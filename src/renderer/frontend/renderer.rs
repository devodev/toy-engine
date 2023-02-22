use std::{io::Cursor, mem, time};

use ash::vk;
use cgmath::{Matrix4, SquareMatrix, Vector3, Vector4};
use log::debug;

use crate::object::GameObject;
use crate::offset_of;
use crate::renderer::backend::buffer::Buffer;
use crate::renderer::backend::descriptor::{DescriptorPool, DescriptorSet, DescriptorSetLayout};
use crate::renderer::backend::device::Device;
use crate::renderer::backend::pipeline::Pipeline;
use crate::renderer::backend::renderpass::RenderPass;
use crate::renderer::backend::shader::Shader;
use crate::Result;
use crate::TIME;

const DEFAULT_MAX_QUADS: u32 = 2000;

#[derive(Clone, Debug)]
struct VertexInputDescription {
    bindings: Vec<vk::VertexInputBindingDescription>,
    attributes: Vec<vk::VertexInputAttributeDescription>,
}

#[derive(Clone, Debug, Copy)]
struct Vertex {
    pos: Vector4<f32>,
    color: Vector4<f32>,
}

impl Vertex {
    fn input_description() -> VertexInputDescription {
        let bindings = vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let attributes = vec![
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Self, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Self, color) as u32,
            },
        ];

        VertexInputDescription {
            bindings,
            attributes,
        }
    }
}

#[derive(Clone, Debug, Copy)]
struct UniformBuffer {
    #[allow(unused)]
    vp: Matrix4<f32>,
}

impl UniformBuffer {
    fn new(vp: Matrix4<f32>) -> Self {
        Self { vp }
    }
}

const QUAD_INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];
const QUAD_VERTICES: [Vector4<f32>; 4] = [
    Vector4::new(-1.0, -1.0, 0.0, 1.0),
    Vector4::new(1.0, -1.0, 0.0, 1.0),
    Vector4::new(1.0, 1.0, 0.0, 1.0),
    Vector4::new(-1.0, 1.0, 0.0, 1.0),
];

#[derive(Debug, Default)]
struct QuadBatchData {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl QuadBatchData {
    fn new(max_quads: u32) -> Self {
        Self {
            vertices: Vec::with_capacity(max_quads as usize * 4),
            indices: Vec::with_capacity(max_quads as usize * 6),
        }
    }

    fn add(&mut self, position: Vector3<f32>, size: Vector3<f32>, color: Vector4<f32>) {
        // compute translation and scale matrices
        let m_translation = Matrix4::from_translation(position);
        let m_scale = Matrix4::from_nonuniform_scale(size.x, size.y, size.z);

        // append indices
        self.indices
            .extend(QUAD_INDICES.iter().map(|i| self.vertices.len() as u32 + i));

        // append vertices
        self.vertices.extend(QUAD_VERTICES.iter().map(|q| Vertex {
            pos: m_scale * m_translation * q,
            color,
        }));
    }
}

#[derive(Debug, Default)]
pub struct QuadBatcher {
    max_quads: u32,
    quad_count: u32,

    current_batch: usize,
    batches: Vec<QuadBatchData>,
}

impl QuadBatcher {
    pub fn new(max_quads: u32) -> Self {
        Self {
            max_quads,
            ..Default::default()
        }
    }

    pub fn add_quad(&mut self, position: Vector3<f32>, size: Vector3<f32>, color: Vector4<f32>) {
        let is_batch_full = self.quad_count == self.max_quads;
        if is_batch_full {
            self.current_batch += 1;
            self.quad_count = 0;
        }
        if is_batch_full || self.batches.is_empty() {
            self.batches.push(QuadBatchData::new(self.max_quads));
        }
        let batch_data = &mut self.batches[self.current_batch];
        batch_data.add(position, size, color);
        self.quad_count += 1;
    }

    pub fn clear(&mut self) {
        self.quad_count = 0;
        self.current_batch = 0;
        self.batches.clear();
    }
}

pub struct Renderer2DSystem {
    /// The vertex and fragment shaders.
    vertex_shader: Shader,
    fragment_shader: Shader,

    // The descriptor pool used to allocate descriptor sets.
    descriptor_pool: DescriptorPool,

    // The descriptor set layout used to allocate descriptor sets.
    descriptor_set_layouts: Vec<DescriptorSetLayout>,
    descriptor_sets: Vec<DescriptorSet>,

    /// Uniform buffer
    uniform_buffer_data: UniformBuffer,
    uniform_buffer: Buffer,

    // Graphics pipeline.
    pipeline: Pipeline,

    // stores quad data
    quad_batcher: QuadBatcher,

    // buffers
    vertex_buffers: Vec<Buffer>,
    index_buffers: Vec<Buffer>,
}

impl Renderer2DSystem {
    /// # Safety
    /// TODO
    pub(crate) unsafe fn new(device: &Device, renderpass: &RenderPass) -> Result<Self> {
        // create shaders
        let mut vertex_spv_file =
            Cursor::new(&include_bytes!("../../../assets/shaders/quad.vert.spv")[..]);
        let mut frag_spv_file =
            Cursor::new(&include_bytes!("../../../assets/shaders/quad.frag.spv")[..]);

        let vertex_shader = Shader::new(device, &mut vertex_spv_file)
            .map_err(|e| format!("create vertex shader module: {:?}", e))?;

        let fragment_shader = Shader::new(device, &mut frag_spv_file)
            .map_err(|e| format!("create fragment shader module: {:?}", e))?;

        // create descriptor pool
        let descriptor_pool_sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
        }];
        let descriptor_pool = DescriptorPool::new(device, &descriptor_pool_sizes, 1)
            .map_err(|e| format!("create descriptor pool: {:?}", e))?;

        // create descriptor sets and layouts
        let (descriptor_sets, descriptor_set_layouts) = {
            let ds_layouts = {
                let ds_layout_bindings = [vk::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    ..Default::default()
                }];
                let ds_layout = DescriptorSetLayout::new(device, &ds_layout_bindings)
                    .map_err(|e| format!("create descriptor set layout: {:?}", e))?;
                vec![ds_layout]
            };
            let ds = DescriptorSet::new(device, &descriptor_pool, &ds_layouts)
                .map_err(|e| format!("create UBO descriptor set: {:?}", e))?;

            (ds, ds_layouts)
        };

        // update descriptor sets
        let (uniform_buffer, uniform_buffer_data, uniform_buffer_data_size) = {
            let buf_data = UniformBuffer::new(Matrix4::identity());
            let buf_size = mem::size_of_val(&buf_data) as u64;
            let mut buf = Buffer::new(
                device,
                device.memory_properties(),
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                buf_size,
            )
            .map_err(|e| format!("create uniform buffer: {:?}", e))?;
            buf.update(device, &[buf_data])
                .map_err(|e| format!("update uniform buffer: {:?}", e))?;
            (buf, buf_data, buf_size)
        };

        descriptor_sets[0]
            .update_ubo(device, &uniform_buffer, 0, uniform_buffer_data_size)
            .map_err(|e| format!("update descriptor set: {:?}", e))?;

        // create graphics pipeline
        let pipeline = {
            let vertex_input_description = Vertex::input_description();
            Pipeline::new(
                device,
                renderpass,
                &vertex_shader,
                &fragment_shader,
                &vertex_input_description.bindings,
                &vertex_input_description.attributes,
                &descriptor_set_layouts,
            )
            .map_err(|e| format!("create pipeline and layout: {:?}", e))?
        };

        // create quad batcher
        let quad_batcher = QuadBatcher::new(DEFAULT_MAX_QUADS);

        Ok(Self {
            vertex_shader,
            fragment_shader,
            descriptor_pool,
            descriptor_set_layouts,
            descriptor_sets,
            uniform_buffer_data,
            uniform_buffer,
            pipeline,
            quad_batcher,
            vertex_buffers: Vec::new(),
            index_buffers: Vec::new(),
        })
    }

    unsafe fn update_uniform_buffer(
        &mut self,
        device: &Device,
        view_projection: Matrix4<f32>,
    ) -> Result<()> {
        TIME!("Renderer2DSystem.update_uniform_buffer");
        self.uniform_buffer_data.vp = view_projection;
        self.uniform_buffer
            .update(device, &[self.uniform_buffer_data])
            .map_err(|e| format!("update uniform buffer: {:?}", e))?;
        Ok(())
    }

    unsafe fn update_buffers(&mut self, device: &Device) -> Result<()> {
        TIME!("Renderer2DSystem.update_buffers");
        for (idx, batch) in self.quad_batcher.batches.iter().enumerate() {
            // vertex buffer
            let vertex_buffer_data = &batch.vertices;
            let vertex_buffer_data_size = mem::size_of_val(&**vertex_buffer_data) as u64;

            // index buffer
            let index_buffer_data = &batch.indices;
            let index_buffer_data_size = mem::size_of_val(&**index_buffer_data) as u64;

            // create buffers if not exists
            let buffer_exists = idx < self.vertex_buffers.len();
            if !buffer_exists {
                // vertex buffer
                let vertex_buffer = Buffer::new(
                    device,
                    device.memory_properties(),
                    vk::BufferUsageFlags::VERTEX_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    vertex_buffer_data_size,
                )
                .map_err(|e| format!("create vertex input buffer: {:?}", e))?;
                self.vertex_buffers.push(vertex_buffer);

                // index buffer
                let index_buffer = Buffer::new(
                    device,
                    device.memory_properties(),
                    vk::BufferUsageFlags::INDEX_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    index_buffer_data_size,
                )
                .map_err(|e| format!("create index buffer: {:?}", e))?;
                self.index_buffers.push(index_buffer);
            }

            // vertex buffer
            self.vertex_buffers
                .get_mut(idx)
                .expect("vertex buffer exists")
                .update(device, vertex_buffer_data)
                .map_err(|e| format!("update vertex buffer: {:?}", e))?;

            // index buffer
            self.index_buffers
                .get_mut(idx)
                .expect("index buffer exists")
                .update(device, index_buffer_data)
                .map_err(|e| format!("update index buffer: {:?}", e))?;
        }
        Ok(())
    }

    /// # Safety
    /// TODO
    pub(crate) unsafe fn render(
        &mut self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        _: time::Duration,
        view_projection: Matrix4<f32>,
        objects: &[GameObject],
    ) -> Result<()> {
        TIME!("Renderer2DSystem.render");
        // update uniform buffer
        self.update_uniform_buffer(device, view_projection)
            .map_err(|e| format!("update uniform buffer: {:?}", e))?;

        // add quads
        for object in objects {
            self.quad_batcher.add_quad(
                object.transform.position,
                object.transform.scale,
                object.color.color,
            );
        }

        // update quad buffers
        self.update_buffers(device)
            .map_err(|e| format!("update quad buffers: {:?}", e))?;

        // record and submit command buffer
        // bind descriptor sets (UBO)
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline.layout,
            0,
            &[*self.descriptor_sets[0]],
            &[],
        );

        // bind pipeline
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            *self.pipeline,
        );

        for (idx, batch) in self.quad_batcher.batches.iter().enumerate() {
            let vertex_buffer = self.vertex_buffers[idx];
            let index_buffer = self.index_buffers[idx];
            let index_count = batch.indices.len() as u32;

            // bind vertex buffers
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[*vertex_buffer], &[0]);

            // bind index buffer
            device.cmd_bind_index_buffer(command_buffer, *index_buffer, 0, vk::IndexType::UINT32);

            // draw
            device.cmd_draw_indexed(command_buffer, index_count, 1, 0, 0, 1);
        }

        // clear quad batcher
        self.quad_batcher.clear();

        Ok(())
    }

    pub(crate) unsafe fn destroy(&mut self, device: &Device) {
        debug!("Destroying Renderer2DSystem");

        // NOTE: All submitted commands that refer to these resources must have
        // completed execution.
        device.device_wait_idle().expect("device wait idle");

        // buffers
        for mut buffer in &mut self.vertex_buffers.drain(..) {
            buffer.destroy(device);
        }
        for mut buffer in &mut self.index_buffers.drain(..) {
            buffer.destroy(device);
        }
        // pipeline
        self.pipeline.destroy(device);
        // uniform buffer
        self.uniform_buffer.destroy(device);
        // descriptor set layout
        for mut layout in &mut self.descriptor_set_layouts.drain(..) {
            layout.destroy(device);
        }
        // descriptor pool
        self.descriptor_pool.destroy(device);
        // shaders
        self.vertex_shader.destroy(device);
        self.fragment_shader.destroy(device);
    }
}
