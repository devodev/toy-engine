use std::{error::Error, ffi::CStr, ops::Deref, result};

use ash::vk;

use super::{descriptor::DescriptorSetLayout, shader::Shader};

type Result<T> = result::Result<T, Box<dyn Error>>;

// Simple offset_of macro akin to C++ offsetof
#[macro_export]
macro_rules! offset_of {
    ($base:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $base = mem::zeroed();
            (&b.$field as *const _ as isize) - (&b as *const _ as isize)
        }
    }};
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct Pipeline {
    pub(crate) handle: vk::Pipeline,
    /// Access to descriptor sets from a pipeline is accomplished through a
    /// pipeline layout. Zero or more descriptor set layouts and zero or more
    /// push constant ranges are combined to form a pipeline layout object
    /// describing the complete set of resources that can be accessed by a
    /// pipeline. The pipeline layout represents a sequence of descriptor sets
    /// with each having a specific layout. This sequence of layouts is used to
    /// determine the interface between shader stages and shader resources. Each
    /// pipeline is created using a pipeline layout.
    pub(crate) layout: vk::PipelineLayout,

    destroyed: bool,
}

impl Pipeline {
    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn new(
        device: &ash::Device,
        renderpass: &vk::RenderPass,
        vertex_shader: &Shader,
        fragment_shader: &Shader,
        vertex_input_binding_descriptions: &[vk::VertexInputBindingDescription],
        vertex_input_attribute_descriptions: &[vk::VertexInputAttributeDescription],
        descriptor_set_layouts: &[DescriptorSetLayout],
    ) -> Result<Self> {
        // shaders
        let shader_stage_create_infos = {
            let shader_entry_name = CStr::from_bytes_with_nul_unchecked(b"main\0");
            [
                vk::PipelineShaderStageCreateInfo::builder()
                    .module(vertex_shader.handle)
                    .name(shader_entry_name)
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .build(),
                vk::PipelineShaderStageCreateInfo::builder()
                    .module(fragment_shader.handle)
                    .name(shader_entry_name)
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .build(),
            ]
        };

        // vertex shader input
        let vertex_input_state_info = {
            vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_binding_descriptions(vertex_input_binding_descriptions)
                .vertex_attribute_descriptions(vertex_input_attribute_descriptions)
        };

        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        // viewport
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1);

        // rasterization
        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE);

        // multisampling
        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // depth stencil
        let depth_state_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL);

        // color blending
        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .build()];
        let color_blend_state_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(&color_blend_attachment_states);

        // mark state as dynamic
        // - the viewport state will be ignored and must be set dynamically using
        //   vkCmdSetViewport
        // - the scissor state will be ignored and must be set dynamically using
        //   vkCmdSetScissor
        //
        // https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkDynamicState.html
        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);

        // create pipeline layout
        let layouts = descriptor_set_layouts
            .iter()
            .map(|d| d.handle)
            .collect::<Vec<_>>();
        let layout_create_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(&layouts);
        let pipeline_layout = device
            .create_pipeline_layout(&layout_create_info, None)
            .map_err(|e| format!("create graphics pipeline layout: {:?}", e))?;

        // create pipeline
        let graphic_pipeline_infos = vk::GraphicsPipelineCreateInfo::builder()
            // what should remain the same between different pipelines
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .viewport_state(&viewport_state_info)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state_info)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(*renderpass)
            // what should change between different pipelines
            .input_assembly_state(&vertex_input_assembly_state_info)
            .rasterization_state(&rasterization_info)
            .build();
        let graphics_pipelines = device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_infos], None)
            .map_err(|e| format!("create graphics pipeline: {:?}", e))?;

        Ok(Self {
            handle: graphics_pipelines[0],
            layout: pipeline_layout,
            destroyed: false,
        })
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        if self.destroyed {
            panic!("pipeline already destroyed")
        }
        device.destroy_pipeline(self.handle, None);
        device.destroy_pipeline_layout(self.layout, None);
        self.destroyed = true;
    }
}

impl Deref for Pipeline {
    type Target = vk::Pipeline;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
