#![allow(unused)]
use std::{error::Error, ops::Deref, result};

use ash::vk;

use super::{device::Device, image::Image};

type Result<T> = result::Result<T, Box<dyn Error>>;

pub(crate) struct RenderPass {
    handle: vk::RenderPass,

    clear_values: Vec<vk::ClearValue>,
}

impl RenderPass {
    pub(crate) unsafe fn new(
        device: &Device,
        image_format: &vk::Format,
        extent: vk::Extent3D,
    ) -> Result<Self> {
        let renderpass = create_renderpass(device, image_format)
            .map_err(|e| format!("create renderpass: {:?}", e))?;

        // renderpass clear values
        let clear_values = vec![
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        Ok(Self {
            handle: renderpass,
            clear_values,
        })
    }

    pub(crate) unsafe fn begin(
        &self,
        device: &ash::Device,
        framebuffer: &vk::Framebuffer,
        render_area: vk::Rect2D,
        command_buffer: &vk::CommandBuffer,
    ) {
        // begin renderpass
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.handle)
            .framebuffer(*framebuffer)
            .render_area(render_area)
            .clear_values(&self.clear_values);
        device.cmd_begin_render_pass(
            *command_buffer,
            &render_pass_begin_info,
            vk::SubpassContents::INLINE,
        );
    }

    pub(crate) unsafe fn end(&self, device: &ash::Device, command_buffer: &vk::CommandBuffer) {
        device.cmd_end_render_pass(*command_buffer);
    }
}

impl Deref for RenderPass {
    type Target = vk::RenderPass;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

unsafe fn create_renderpass(
    device: &Device,
    color_image_format: &vk::Format,
) -> Result<vk::RenderPass> {
    let renderpass_attachments = [
        // Color
        vk::AttachmentDescription {
            format: *color_image_format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        },
        // Depth
        vk::AttachmentDescription {
            format: vk::Format::D16_UNORM,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ..Default::default()
        },
    ];

    let subpasses = {
        let color_attachments = &[vk::AttachmentReference {
            // Color
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let depth_attachment = vk::AttachmentReference {
            // Depth
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };

        let subpass = vk::SubpassDescription::builder()
            // The index of the attachment in this array is directly referenced from the
            // fragment shader with the layout(location = 0) out vec4 outColor directive!
            // .input_attachments(input_attachments)
            .color_attachments(color_attachments)
            .depth_stencil_attachment(&depth_attachment)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .build();

        [subpass]
    };

    let dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
            | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
            | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE
            | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        ..Default::default()
    }];

    let renderpass_create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&renderpass_attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    let renderpass = device
        .create_render_pass(&renderpass_create_info, None)
        .map_err(|e| format!("create renderpass: {:?}", e))?;

    Ok(renderpass)
}
