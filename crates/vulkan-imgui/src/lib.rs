#![allow(clippy::missing_safety_doc)]

///! Inspired by:
///! https://github.com/Yatekii/imgui-wgpu-rs/blob/master/src/lib.rs
///! https://github.com/unknownue/vulkan-tutorial-rust/blob/master/src/tutorials/23_texture_image.rs
///! https://github.com/adrien-ben/imgui-rs-vulkan-renderer/blob/master/src/renderer/vulkan.rs
use std::io::Cursor;
use std::mem;
use std::ops::Deref;
use std::{error, result};

use ash::vk;
use cgmath::{Matrix4, SquareMatrix};
use imgui::DrawCmd::Elements;
use imgui::{DrawData, DrawIdx, DrawList, DrawVert, FontConfig};
use log::debug;
use vulkan_renderer::buffer::Buffer;
use vulkan_renderer::descriptor::{DescriptorPool, DescriptorSet, DescriptorSetLayout};
use vulkan_renderer::device::Device;
use vulkan_renderer::image::Image;
use vulkan_renderer::offset_of;
use vulkan_renderer::pipeline::Pipeline;
use vulkan_renderer::renderpass::RenderPass;
use vulkan_renderer::shader::Shader;
use vulkan_renderer::texture::Texture;
use winit::window::Window;

type Result<T> = result::Result<T, Box<dyn error::Error>>;

pub fn init(window: &Window) -> (imgui_winit_support::WinitPlatform, imgui::Context) {
    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);

    let mut winit_platform = imgui_winit_support::WinitPlatform::init(&mut imgui_context);

    let hidpi_factor = winit_platform.hidpi_factor();
    let font_size = (13.0 * hidpi_factor) as f32;
    imgui_context
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData {
            config: Some(FontConfig {
                size_pixels: font_size,
                ..imgui::FontConfig::default()
            }),
        }]);

    let dpi_mode = imgui_winit_support::HiDpiMode::Rounded;
    winit_platform.attach_window(imgui_context.io_mut(), window, dpi_mode);

    (winit_platform, imgui_context)
}

struct VertexInputDescription {
    bindings: Vec<vk::VertexInputBindingDescription>,
    attributes: Vec<vk::VertexInputAttributeDescription>,
}

#[repr(transparent)]
#[derive(Clone, Debug, Copy)]
struct Vertex(DrawVert);

impl Deref for Vertex {
    type Target = DrawVert;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
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
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Self, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Self, uv) as u32,
            },
            // ImGui outputs SRGB colors ([u8; 4])
            // We convert them to linear space in the shader
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R8G8B8A8_UNORM,
                offset: offset_of!(Self, col) as u32,
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
    ortho: Matrix4<f32>,
}

impl UniformBuffer {
    fn new(ortho: Matrix4<f32>) -> Self {
        Self { ortho }
    }
}

pub struct RenderData {
    fb_size: [f32; 2],
    last_size: [f32; 2],
    last_pos: [f32; 2],
    vertex_buffer: Option<Buffer>,
    vertex_buffer_size: usize,
    index_buffer: Option<Buffer>,
    index_buffer_size: usize,
    draw_list_offsets: Vec<(i32, u32)>,
    render: bool,
}

pub struct Renderer {
    /// The vertex and fragment shaders
    vertex_shader: Shader,
    fragment_shader: Shader,

    // The descriptor pool used to allocate descriptor sets
    descriptor_pool: DescriptorPool,

    // The descriptor set layout used to allocate descriptor sets
    descriptor_set_layouts: Vec<DescriptorSetLayout>,
    descriptor_sets: Vec<DescriptorSet>,

    /// Uniform buffer
    uniform_buffer: Buffer,

    // Command Pool
    command_pool: vk::CommandPool,

    // Graphics pipeline
    pipeline: Pipeline,

    render_data: Option<RenderData>,

    textures: imgui::Textures<Texture>,
}

impl Renderer {
    pub unsafe fn new(
        ctx: &mut imgui::Context,
        device: &Device,
        renderpass: &RenderPass,
    ) -> Result<Self> {
        // create shaders
        let (vertex_shader, fragment_shader) = {
            let mut vert_file = Cursor::new(&include_bytes!("../shaders/imgui.vert.spv")[..]);
            let mut frag_file = Cursor::new(&include_bytes!("../shaders/imgui.frag.spv")[..]);

            let vert = Shader::new(device, &mut vert_file)
                .map_err(|e| format!("create vertex shader module: {:?}", e))?;
            let frag = Shader::new(device, &mut frag_file)
                .map_err(|e| format!("create fragment shader module: {:?}", e))?;

            (vert, frag)
        };

        // create uniform buffer
        let (uniform_buffer, uniform_buffer_data_size) = {
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
            (buf, buf_size)
        };

        // create command pool
        let command_pool = device
            .create_command_pool()
            .map_err(|e| format!("create command buffer pool: {:?}", e))?;

        // create imgui font texture
        let mut textures = imgui::Textures::new();
        let font_tex_id = reload_font_texture(device, ctx, &command_pool, &mut textures)
            .map_err(|e| format!("load font texture: {:?}", e))?;
        let font_tex = textures
            .get(font_tex_id)
            .expect("imgui font texture exists");

        // create descriptor pool
        let descriptor_pool = {
            let descriptor_pool_sizes = [
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1,
                },
            ];
            DescriptorPool::new(device, &descriptor_pool_sizes, 1)
                .map_err(|e| format!("create descriptor pool: {:?}", e))?
        };

        // create descriptor sets and layouts
        let (descriptor_sets, descriptor_set_layouts) = {
            let ds_layouts = {
                let ds_layout_bindings = [
                    vk::DescriptorSetLayoutBinding {
                        binding: 0,
                        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                        descriptor_count: 1,
                        stage_flags: vk::ShaderStageFlags::VERTEX,
                        ..Default::default()
                    },
                    vk::DescriptorSetLayoutBinding {
                        binding: 1,
                        descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                        descriptor_count: 1,
                        stage_flags: vk::ShaderStageFlags::FRAGMENT,
                        ..Default::default()
                    },
                ];
                let ds_layout = DescriptorSetLayout::new(device, &ds_layout_bindings)
                    .map_err(|e| format!("create descriptor set layout: {:?}", e))?;
                vec![ds_layout]
            };
            let ds = DescriptorSet::new(device, &descriptor_pool, &ds_layouts)
                .map_err(|e| format!("create UBO descriptor set: {:?}", e))?;

            (ds, ds_layouts)
        };

        let descriptor_set = descriptor_sets[0];

        let buffer_info = vk::DescriptorBufferInfo {
            buffer: *uniform_buffer.buffer(),
            range: uniform_buffer_data_size,
            offset: 0,
        };
        let image_info = vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: *font_tex.image_view(),
            sampler: **font_tex.sampler(),
        };
        let descriptor_writes = &[
            vk::WriteDescriptorSet {
                dst_set: *descriptor_set,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_buffer_info: &buffer_info,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: *descriptor_set,
                dst_binding: 1,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &image_info,
                ..Default::default()
            },
        ];
        descriptor_set
            .update(device, descriptor_writes)
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

        let renderer = Self {
            vertex_shader,
            fragment_shader,
            descriptor_pool,
            descriptor_set_layouts,
            descriptor_sets,
            uniform_buffer,
            command_pool,
            pipeline,
            render_data: None,
            textures,
        };

        Ok(renderer)
    }

    pub fn prepare(
        &mut self,
        device: &Device,
        draw_data: &DrawData,
        render_data: Option<RenderData>,
    ) -> Result<RenderData> {
        let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];

        let mut render_data = render_data.unwrap_or_else(|| RenderData {
            fb_size: [fb_width, fb_height],
            last_size: [0.0, 0.0],
            last_pos: [0.0, 0.0],
            vertex_buffer: None,
            vertex_buffer_size: 0,
            index_buffer: None,
            index_buffer_size: 0,
            draw_list_offsets: Vec::new(),
            render: false,
        });

        // If the render area is <= 0, exit here and now.
        if fb_width <= 0.0 || fb_height <= 0.0 || draw_data.draw_lists_count() == 0 {
            render_data.render = false;
            return Ok(render_data);
        } else {
            render_data.render = true;
        }

        // Only update matrices if the size or position changes
        if (render_data.last_size[0] - draw_data.display_size[0]).abs() > f32::EPSILON
            || (render_data.last_size[1] - draw_data.display_size[1]).abs() > f32::EPSILON
            || (render_data.last_pos[0] - draw_data.display_pos[0]).abs() > f32::EPSILON
            || (render_data.last_pos[1] - draw_data.display_pos[1]).abs() > f32::EPSILON
        {
            render_data.fb_size = [fb_width, fb_height];
            render_data.last_size = draw_data.display_size;
            render_data.last_pos = draw_data.display_pos;

            let width = draw_data.display_size[0];
            let height = draw_data.display_size[1];

            // Create and update the transform matrix for the current frame.
            // This is required to adapt to vulkan coordinates.
            unsafe {
                let ortho = cgmath::ortho(0.0, width, 0.0, height, -1.0, 1.0);
                let ubo = UniformBuffer::new(ortho);
                self.uniform_buffer
                    .update(device, &[ubo])
                    .map_err(|e| format!("update uniform buffer: {:?}", e))?;
            }
        }

        render_data.draw_list_offsets.clear();

        let mut vertex_count = 0;
        let mut index_count = 0;
        for draw_list in draw_data.draw_lists() {
            render_data
                .draw_list_offsets
                .push((vertex_count as i32, index_count as u32));
            vertex_count += draw_list.vtx_buffer().len();
            index_count += draw_list.idx_buffer().len();
        }

        let mut vertex_buffer_data =
            Vec::with_capacity(vertex_count * std::mem::size_of::<Vertex>());
        let mut index_buffer_data =
            Vec::with_capacity(index_count * std::mem::size_of::<DrawIdx>());

        for draw_list in draw_data.draw_lists() {
            // Safety: Vertex is #[repr(transparent)] over DrawVert.
            let vertex_data: &[Vertex] = unsafe { draw_list.transmute_vtx_buffer() };
            vertex_buffer_data.extend_from_slice(vertex_data);
            index_buffer_data.extend_from_slice(draw_list.idx_buffer());
        }

        // If the buffer is not created or is too small for the new indices, create a
        // new buffer
        if render_data.index_buffer.is_none()
            || render_data.index_buffer_size < index_buffer_data.len()
        {
            unsafe {
                let index_buffer_data_size = mem::size_of_val(&*index_buffer_data) as u64;
                let mut index_buffer = Buffer::new(
                    device,
                    device.memory_properties(),
                    vk::BufferUsageFlags::INDEX_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    index_buffer_data_size,
                )
                .map_err(|e| format!("create index buffer: {:?}", e))?;
                index_buffer
                    .update(device, &index_buffer_data)
                    .map_err(|e| format!("update index buffer: {:?}", e))?;

                if let Some(mut index_buffer) = render_data.index_buffer.take() {
                    device.device_wait_idle().expect("device wait idle");
                    index_buffer.destroy(device);
                }
                render_data.index_buffer = Some(index_buffer);
                render_data.index_buffer_size = index_buffer_data.len();
            }
        } else if let Some(buffer) = render_data.index_buffer.as_mut() {
            // The buffer is large enough for the new indices, so reuse it
            unsafe {
                buffer
                    .update(device, &index_buffer_data)
                    .map_err(|e| format!("update index buffer: {:?}", e))?
            }
        } else {
            unreachable!()
        }

        // If the buffer is not created or is too small for the new vertices, create a
        // new buffer
        if render_data.vertex_buffer.is_none()
            || render_data.vertex_buffer_size < vertex_buffer_data.len()
        {
            unsafe {
                let vertex_buffer_data_size = mem::size_of_val(&*vertex_buffer_data) as u64;
                let mut vertex_buffer = Buffer::new(
                    device,
                    device.memory_properties(),
                    vk::BufferUsageFlags::VERTEX_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    vertex_buffer_data_size,
                )
                .map_err(|e| format!("create vertex buffer: {:?}", e))?;
                vertex_buffer
                    .update(device, &vertex_buffer_data)
                    .map_err(|e| format!("update vertex buffer: {:?}", e))?;

                if let Some(mut vertex_buffer) = render_data.vertex_buffer.take() {
                    device.device_wait_idle().expect("device wait idle");
                    vertex_buffer.destroy(device);
                }
                render_data.vertex_buffer = Some(vertex_buffer);
                render_data.vertex_buffer_size = vertex_buffer_data.len();
            }
        } else if let Some(buffer) = render_data.vertex_buffer.as_mut() {
            // The buffer is large enough for the new indices, so reuse it
            unsafe {
                buffer
                    .update(device, &vertex_buffer_data)
                    .map_err(|e| format!("update vertex buffer: {:?}", e))?;
            }
        } else {
            unreachable!()
        }

        Ok(render_data)
    }

    pub unsafe fn render(
        &mut self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        draw_data: &DrawData,
    ) -> Result<()> {
        let render_data = self.render_data.take();
        let render_data = Some(self.prepare(device, draw_data, render_data)?);
        self.split_render(
            device,
            command_buffer,
            draw_data,
            render_data.as_ref().unwrap(),
        )?;
        self.render_data = render_data;

        Ok(())
    }

    pub unsafe fn split_render(
        &mut self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        draw_data: &DrawData,
        render_data: &RenderData,
    ) -> Result<()> {
        if !render_data.render {
            return Ok(());
        }

        // bind descriptor sets
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

        // bind vertex buffers
        let vertex_buffer = render_data.vertex_buffer.expect("vertex buffer is set");
        device.cmd_bind_vertex_buffers(command_buffer, 0, &[*vertex_buffer], &[0]);

        // bind index buffer
        let index_buffer = render_data.index_buffer.expect("index buffer is set");
        device.cmd_bind_index_buffer(command_buffer, *index_buffer, 0, vk::IndexType::UINT16);

        // Execute all the imgui render work.
        for (draw_list, bases) in draw_data
            .draw_lists()
            .zip(render_data.draw_list_offsets.iter())
        {
            self.render_draw_list(
                device,
                command_buffer,
                draw_list,
                render_data.fb_size,
                draw_data.display_pos,
                draw_data.framebuffer_scale,
                *bases,
            )?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub unsafe fn render_draw_list(
        &mut self,
        device: &Device,
        command_buffer: vk::CommandBuffer,
        draw_list: &DrawList,
        fb_size: [f32; 2],
        clip_off: [f32; 2],
        clip_scale: [f32; 2],
        (vertex_base, index_base): (i32, u32),
    ) -> Result<()> {
        let mut start = index_base;

        for cmd in draw_list.commands() {
            if let Elements { count, cmd_params } = cmd {
                let clip_rect = [
                    (cmd_params.clip_rect[0] - clip_off[0]) * clip_scale[0],
                    (cmd_params.clip_rect[1] - clip_off[1]) * clip_scale[1],
                    (cmd_params.clip_rect[2] - clip_off[0]) * clip_scale[0],
                    (cmd_params.clip_rect[3] - clip_off[1]) * clip_scale[1],
                ];

                let end = start + count as u32;

                if clip_rect[0] < fb_size[0]
                    && clip_rect[1] < fb_size[1]
                    && clip_rect[2] >= 0.0
                    && clip_rect[3] >= 0.0
                {
                    // set scissor
                    let scissor = vk::Rect2D {
                        offset: vk::Offset2D {
                            x: clip_rect[0].max(0.0).floor() as i32,
                            y: clip_rect[1].max(0.0).floor() as i32,
                        },
                        extent: vk::Extent2D {
                            width: (clip_rect[2] - clip_rect[0]).abs().ceil() as u32,
                            height: (clip_rect[3] - clip_rect[1]).abs().ceil() as u32,
                        },
                    };
                    device.cmd_set_scissor(command_buffer, 0, &[scissor]);
                    device.cmd_draw_indexed(command_buffer, count as u32, 1, start, vertex_base, 0);
                }

                // Increment the index regardless of whether or not this batch
                // of vertices was drawn.
                start = end;
            }
        }

        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &Device, ctx: &mut imgui::Context) {
        debug!("Destroying imgui::Renderer");

        // NOTE: All submitted commands that refer to these resources must have
        // completed execution.
        device.device_wait_idle().expect("device wait idle");

        // buffers
        if let Some(render_data) = self.render_data.take() {
            if let Some(mut buf) = render_data.vertex_buffer {
                buf.destroy(device);
            }
            if let Some(mut buf) = render_data.index_buffer {
                buf.destroy(device);
            }
        }
        // pipeline
        self.pipeline.destroy(device);
        // command pool
        device.destroy_command_pool(self.command_pool, None);
        // uniform buffer
        self.uniform_buffer.destroy(device);
        // font atlas texture
        if let Some(mut tex) = self.textures.remove(ctx.fonts().tex_id) {
            tex.destroy(device);
        }
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

/// Updates the texture on the GPU corresponding to the current imgui font
/// atlas.
///
/// This has to be called after loading a font.
pub unsafe fn reload_font_texture(
    device: &Device,
    ctx: &mut imgui::Context,
    command_pool: &vk::CommandPool,
    textures: &mut imgui::Textures<Texture>,
) -> Result<imgui::TextureId> {
    let mut fonts = ctx.fonts();
    // Remove possible font atlas texture.
    if let Some(mut tex) = textures.remove(fonts.tex_id) {
        tex.destroy(device);
    }

    // Create font texture and upload it.
    let handle = fonts.build_rgba32_texture();

    let create_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_UNORM)
        .extent(vk::Extent3D {
            width: handle.width,
            height: handle.height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let mut font_image = Image::new(
        device,
        device.memory_properties(),
        *create_info,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    font_image
        .upload_gpu(device, *command_pool, handle.data)
        .map_err(|e| format!("update font texture data: {:?}", e))?;
    let font_texture = Texture::from_image(device, font_image)?;
    fonts.tex_id = textures.insert(font_texture);

    // Clear imgui texture data to save memory.
    fonts.clear_tex_data();

    Ok(fonts.tex_id)
}
