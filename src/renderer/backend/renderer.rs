use ash::vk;
use log::debug;
use winit::window::Window;

use super::device::Device;
use super::image::Image;
use super::swapchain::Swapchain;
use crate::renderer::backend::renderpass::RenderPass;
use crate::Result;

/// Number of frames in flight at any moment. This is used to isolate rendering
/// logic related to each frame. It includes command buffers and semaphores.
const MAX_FRAMES_IN_FLIGHT: u32 = 2;

struct FrameData {
    /// Fences are a synchronization primitive that can be used to insert a
    /// dependency from a queue to the host.
    render_fence: vk::Fence,

    /// Semaphores are a synchronization primitive that can be used to insert a
    /// dependency between queue operations or between a queue operation and the
    /// host.
    present_semaphore: vk::Semaphore,
    render_semaphore: vk::Semaphore,

    /// Primary command buffer object used to record commands for this frame.
    command_buffer: vk::CommandBuffer,
}

impl FrameData {
    unsafe fn new(device: &Device, command_pool: &vk::CommandPool) -> Result<Self> {
        // create fence
        let present_fence = {
            let fence_create_info =
                vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
            device
                .create_fence(&fence_create_info, None)
                .map_err(|e| format!("create fence: {:?}", e))?
        };

        // create semaphores
        let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
        let present_semaphore = device
            .create_semaphore(&semaphore_create_info, None)
            .map_err(|e| format!("create semaphore: {:?}", e))?;
        let render_semaphore = device
            .create_semaphore(&semaphore_create_info, None)
            .map_err(|e| format!("create semaphore: {:?}", e))?;

        // create command buffer
        let command_buffer = {
            device
                .create_command_buffers(command_pool, 1)
                .map_err(|e| format!("create command buffers: {:?}", e))?[0]
        };

        Ok(Self {
            render_fence: present_fence,
            present_semaphore,
            render_semaphore,
            command_buffer,
        })
    }

    unsafe fn destroy(&mut self, device: &Device) {
        // semaphores
        device.destroy_semaphore(self.present_semaphore, None);
        device.destroy_semaphore(self.render_semaphore, None);
        // fences
        device.destroy_fence(self.render_fence, None);
    }
}

pub struct VulkanRenderer {
    /// The device is the interface used to talk to Vulkan.
    device: Device,

    /// The swapchain holds the images we will draw onto.
    swapchain: Swapchain,

    /// All rendering happens in the context of a renderpass.
    renderpass: RenderPass,

    /// Command pools are opaque objects that command buffer memory is allocated
    /// from, and which allow the implementation to amortize the cost of
    /// resource creation across multiple command buffers. Command pools are
    /// externally synchronized, meaning that a command pool must not be used
    /// concurrently in multiple threads. That includes use via recording
    /// commands on any command buffers allocated from the pool, as well as
    /// operations that allocate, free, and reset command buffers or the pool
    /// itself.
    command_pool: vk::CommandPool,

    /// Manages command pool/buffers and fences/semaphores used for rendering.
    frames: Vec<FrameData>,
    frame_number: u32,
    max_frames_in_flight: u32,

    /// depth image used in RenderPass
    depth_image: Image,
    depth_image_view: vk::ImageView,

    /// Framebuffers holds buffers for drawing.
    framebuffers: Vec<vk::Framebuffer>,

    /// A two-dimensional extent representing the size of the surface.
    window_extent: vk::Extent2D,

    /// Set to true when the surface_resolution has been updated.
    framebuffer_resized: bool,

    /// Indicate wheter a frame has been started using begin_frame().
    frame_started: bool,
}

impl VulkanRenderer {
    /// Creates a new Vulkan context.
    ///
    /// # Safety
    /// NOTHING IS SAFE HERE, GLHF
    pub unsafe fn new(app_name: impl AsRef<str>, window: &Window) -> Result<Self> {
        // create device
        let device =
            Device::new(app_name, window).map_err(|e| format!("create device: {:?}", e))?;

        let window_extent = {
            let window_size = window.inner_size();
            vk::Extent2D {
                width: window_size.width,
                height: window_size.height,
            }
        };

        // create command pool
        let command_pool = device
            .create_command_pool()
            .map_err(|e| format!("create command buffer pool: {:?}", e))?;

        // create fame data
        let max_frames_in_flight = MAX_FRAMES_IN_FLIGHT;
        let mut frames = Vec::with_capacity(max_frames_in_flight as usize);
        for _ in 0..max_frames_in_flight {
            let frame_data = FrameData::new(&device, &command_pool)
                .map_err(|e| format!("create frame data: {:?}", e))?;
            frames.push(frame_data);
        }

        // create swapchain
        let swapchain = Swapchain::new(&device, window_extent)
            .map_err(|e| format!("create swapchain: {:?}", e))?;

        // create renderpass
        let renderpass = RenderPass::new(&device, swapchain.image_format())
            .map_err(|e| format!("create renderpass: {:?}", e))?;

        // create depth image
        let depth_image = create_depth_image(&device, window_extent.into())
            .map_err(|e| format!("create depth image: {:?}", e))?;

        // create depth image view used for writing depth data
        let depth_image_view =
            create_depth_image_view(&device, depth_image.image(), depth_image.format())
                .map_err(|e| format!("create depth image view: {:?}", e))?;

        // create framebuffers
        let framebuffers = create_framebuffers(
            &device,
            &renderpass,
            swapchain.image_views(),
            &depth_image_view,
            window_extent,
        )
        .map_err(|e| format!("create framebuffers: {:?}", e))?;

        let renderer = Self {
            device,
            window_extent,
            command_pool,
            frames,
            frame_number: 0,
            max_frames_in_flight,
            swapchain,
            renderpass,
            depth_image,
            depth_image_view,
            framebuffers,
            framebuffer_resized: false,
            frame_started: false,
        };

        Ok(renderer)
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.window_extent = vk::Extent2D { width, height };
        self.framebuffer_resized = true;
    }

    pub(crate) unsafe fn begin_frame(&mut self) -> Result<bool> {
        // do not render if we are minimized or window is reduced to 0 in any direction
        if self.window_extent.width == 0 || self.window_extent.height == 0 {
            return Ok(false);
        }

        let frame_data = self.current_frame();
        let timeout = std::u64::MAX;

        // wait and reset fences
        {
            let wait_all = true;
            let fences = [frame_data.render_fence];
            self.device
                .wait_for_fences(&fences, wait_all, timeout)
                .map_err(|e| format!("wait for fences: {:?}", e))?;
            self.device
                .reset_fences(&fences)
                .map_err(|e| format!("reset fences: {:?}", e))?;
        }

        // acquire next image
        let suboptimal = {
            let present_semaphore = frame_data.present_semaphore;
            let render_fence = frame_data.render_fence;
            self.swapchain
                .acquire_next_image(timeout, &present_semaphore, &render_fence)
                .map_err(|e| format!("acquire next image: {:?}", e))?
        };

        // recreate swapchain if needed
        if suboptimal || self.framebuffer_resized {
            self.framebuffer_resized = false;
            self.recreate_swapchain()
                .map_err(|e| format!("recreate swapchain: {:?}", e))?;
            return Ok(false);
        }

        self.frame_started = true;

        Ok(true)
    }

    pub(crate) unsafe fn end_frame(&mut self) -> Result<bool> {
        if !self.frame_started {
            return Err("submit_frame called but frame has not been prepared".into());
        }

        let frame_data = self.current_frame();

        // queue image for presentation
        let wait_semaphores = [frame_data.render_semaphore];
        let suboptimal = self
            .swapchain
            .queue_present(&self.device, &wait_semaphores)
            .map_err(|e| format!("queue present: {:?}", e))?;

        // recreate swapchain if needed
        if suboptimal {
            self.recreate_swapchain()
                .map_err(|e| format!("recreate swapchain: {:?}", e))?;
            return Ok(false);
        }

        self.frame_started = false;
        self.bump_frame();

        Ok(true)
    }

    pub(crate) unsafe fn draw<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
        &self,
        f: F,
    ) -> Result<()> {
        if !self.frame_started {
            return Err("draw_and_submit called but frame has not been started".into());
        }

        let frame_data = self.current_frame();

        self.immediate_submit(
            frame_data.command_buffer,
            frame_data.render_fence,
            frame_data.render_semaphore,
            frame_data.present_semaphore,
            |device, cb| {
                // begin renderpass
                let framebuffer = self.current_framebuffer();
                self.renderpass
                    .begin(device, framebuffer, self.window_extent.into(), &cb);

                // set viewport and scissor
                // NOTE: needed because we've set these as dynamic attributes
                let (viewport, scissor) = create_viewport_and_scissor(self.window_extent);
                device.cmd_set_viewport(cb, 0, &[viewport]);
                device.cmd_set_scissor(cb, 0, &[scissor]);

                // do the actual command buffer recording from the closure
                f(device, cb);

                // end renderpass
                self.renderpass.end(device, &cb);
            },
        )
        .map_err(|e| format!("immediate submit: {:?}", e))?;

        Ok(())
    }

    pub(crate) fn device(&self) -> &Device {
        &self.device
    }

    pub(crate) fn renderpass(&self) -> &RenderPass {
        &self.renderpass
    }

    #[allow(unused)]
    pub(crate) unsafe fn destroy_image(&self, image: &mut Image) {
        image.destroy(&self.device)
    }

    unsafe fn recreate_swapchain(&mut self) -> Result<()> {
        // ensure all operations on the device have been finished before destroying
        // resources
        self.device
            .device_wait_idle()
            .map_err(|e| format!("device wait idle: {:?}", e))?;

        /////////////////////////////////////////
        // destroy swapchain-related components
        /////////////////////////////////////////

        // framebuffers
        for framebuffer in self.framebuffers.drain(..) {
            self.device.destroy_framebuffer(framebuffer, None);
        }
        // depth image
        self.device.destroy_image_view(self.depth_image_view, None);
        self.depth_image.destroy(&self.device);
        // renderpass
        self.device.destroy_render_pass(*self.renderpass, None);
        // swapchain
        self.swapchain.destroy(&self.device);

        /////////////////////////////////////////
        // recreate swapchain
        /////////////////////////////////////////

        let swapchain = Swapchain::new(&self.device, self.window_extent)
            .map_err(|e| format!("recreate swapchain: {:?}", e))?;

        // create renderpass
        let renderpass = RenderPass::new(&self.device, swapchain.image_format())
            .map_err(|e| format!("create renderpass: {:?}", e))?;

        // create depth image
        let depth_image = create_depth_image(&self.device, self.window_extent.into())
            .map_err(|e| format!("create depth image: {:?}", e))?;

        let depth_image_view =
            create_depth_image_view(&self.device, depth_image.image(), depth_image.format())
                .map_err(|e| format!("create depth image view: {:?}", e))?;

        // create framebuffers
        let framebuffers = create_framebuffers(
            &self.device,
            &renderpass,
            swapchain.image_views(),
            &depth_image_view,
            self.window_extent,
        )
        .map_err(|e| format!("create framebuffers: {:?}", e))?;

        /////////////////////////////////////////
        // set swapchain
        /////////////////////////////////////////

        self.swapchain = swapchain;
        self.renderpass = renderpass;
        self.depth_image = depth_image;
        self.depth_image_view = depth_image_view;
        self.framebuffers = framebuffers;

        Ok(())
    }

    fn current_framebuffer(&self) -> &vk::Framebuffer {
        let image_index = self.swapchain.current_index();
        &self.framebuffers[image_index]
    }

    fn current_frame(&self) -> &FrameData {
        let idx = self.frame_number % self.max_frames_in_flight;
        &self.frames[idx as usize]
    }

    fn bump_frame(&mut self) {
        self.frame_number += 1;
    }

    unsafe fn immediate_submit<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
        &self,
        command_buffer: vk::CommandBuffer,
        render_fence: vk::Fence,
        render_semaphore: vk::Semaphore,
        present_semaphore: vk::Semaphore,
        f: F,
    ) -> Result<()> {
        immediate_submit(
            &self.device,
            command_buffer,
            render_fence,
            render_semaphore,
            present_semaphore,
            f,
        )
    }

    pub(crate) unsafe fn destroy(&mut self) {
        debug!("Destroying Vulkan Renderer");

        // Wait for a device to become idle (completion of outstanding queue operations
        // for all queues on a given logical device).
        self.device.device_wait_idle().expect("device wait idle");
        // framebuffers
        for framebuffer in self.framebuffers.drain(..) {
            self.device.destroy_framebuffer(framebuffer, None);
        }
        // depth image
        self.device.destroy_image_view(self.depth_image_view, None);
        self.depth_image.destroy(&self.device);
        // renderpass
        self.device.destroy_render_pass(*self.renderpass, None);
        // swapchain
        self.swapchain.destroy(&self.device);
        for mut frame_data in self.frames.drain(..) {
            frame_data.destroy(&self.device);
        }
        // command buffers
        self.device.destroy_command_pool(self.command_pool, None);
        // device
        self.device.destroy();
    }
}

pub(crate) unsafe fn copy_buffer_to_image(
    device: &Device,
    command_pool: vk::CommandPool,
    buffer: vk::Buffer,
    image: vk::Image,
    width: u32,
    height: u32,
) -> Result<()> {
    let buffer_image_regions = [vk::BufferImageCopy {
        image_subresource: vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        },
        image_extent: vk::Extent3D {
            width,
            height,
            depth: 1,
        },
        buffer_offset: 0,
        buffer_image_height: 0,
        buffer_row_length: 0,
        image_offset: vk::Offset3D::default(),
    }];

    single_time_command(device, command_pool, |device, command_buffer| {
        device.cmd_copy_buffer_to_image(
            command_buffer,
            buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &buffer_image_regions,
        );
    })
}

pub(crate) unsafe fn transition_image_layout(
    device: &Device,
    command_pool: vk::CommandPool,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> Result<()> {
    let src_access_mask;
    let dst_access_mask;
    let source_stage;
    let destination_stage;

    if old_layout == vk::ImageLayout::UNDEFINED
        && new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
    {
        src_access_mask = vk::AccessFlags::empty();
        dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
        destination_stage = vk::PipelineStageFlags::TRANSFER;
    } else if old_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
        && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
    {
        src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        dst_access_mask = vk::AccessFlags::SHADER_READ;
        source_stage = vk::PipelineStageFlags::TRANSFER;
        destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
    } else {
        return Err("Unsupported layout transition!".into());
    }

    let image_barriers = &[vk::ImageMemoryBarrier::builder()
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask)
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(
            vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1)
                .build(),
        )
        .build()];

    single_time_command(device, command_pool, |device, command_buffer| {
        device.cmd_pipeline_barrier(
            command_buffer,
            source_stage,
            destination_stage,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            image_barriers,
        );
    })
}

pub(crate) unsafe fn single_time_command<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
    device: &Device,
    command_pool: vk::CommandPool,
    f: F,
) -> Result<()> {
    // create command buffer
    let command_buffer = device
        .create_command_buffers(&command_pool, 1)
        .map_err(|e| format!("create command buffer: {:?}", e))?[0];

    // record command buffer
    record_commandbuffer(device, command_buffer, f)
        .map_err(|e| format!("record commandbuffer: {:?}", e))?;

    // prepare submits
    let submits = [vk::SubmitInfo::builder()
        .command_buffers(&[command_buffer])
        .build()];

    // submit command buffer to queue
    device
        .queue_submit(*device.graphics_queue(), &submits, vk::Fence::null())
        .map_err(|e| format!("queue submit: {:?}", e))?;
    Ok(())
}

unsafe fn immediate_submit<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    render_fence: vk::Fence,
    render_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,
    f: F,
) -> Result<()> {
    // record command buffer
    record_commandbuffer(device, command_buffer, f)
        .map_err(|e| format!("record commandbuffer: {:?}", e))?;

    // wait and reset fences
    device
        .wait_for_fences(&[render_fence], true, std::u64::MAX)
        .map_err(|e| format!("wait for fences: {:?}", e))?;
    device
        .reset_fences(&[render_fence])
        .map_err(|e| format!("reset fences: {:?}", e))?;

    // prepare submits
    let submits = [vk::SubmitInfo::builder()
        .wait_semaphores(&[present_semaphore])
        .wait_dst_stage_mask(&[vk::PipelineStageFlags::BOTTOM_OF_PIPE])
        .command_buffers(&[command_buffer])
        .signal_semaphores(&[render_semaphore])
        .build()];

    // submit command buffer to queue
    device
        .queue_submit(*device.graphics_queue(), &submits, render_fence)
        .map_err(|e| format!("queue submit: {:?}", e))?;
    Ok(())
}

unsafe fn record_commandbuffer<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    f: F,
) -> Result<()> {
    // begin command buffer
    let command_buffer_begin_info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    device
        .begin_command_buffer(command_buffer, &command_buffer_begin_info)
        .map_err(|e| format!("begin commandbuffer: {:?}", e))?;

    // record command buffer
    f(device, command_buffer);

    // end command buffer
    device
        .end_command_buffer(command_buffer)
        .map_err(|e| format!("end commandbuffer: {:?}", e))?;

    Ok(())
}

pub(crate) fn create_viewport_and_scissor(extent: vk::Extent2D) -> (vk::Viewport, vk::Rect2D) {
    let viewport = vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: extent.width as f32,
        height: extent.height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    };
    let scissor = extent.into();

    (viewport, scissor)
}

unsafe fn create_framebuffers(
    device: &Device,
    renderpass: &vk::RenderPass,
    present_image_views: &[vk::ImageView],
    depth_image_view: &vk::ImageView,
    surface_resolution: vk::Extent2D,
) -> Result<Vec<vk::Framebuffer>> {
    let mut framebuffers = Vec::new();
    for image_view in present_image_views {
        let framebuffer_attachments = [*image_view, *depth_image_view];
        let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(*renderpass)
            .attachments(&framebuffer_attachments)
            .width(surface_resolution.width)
            .height(surface_resolution.height)
            .layers(1);
        let framebuffer = device
            .create_framebuffer(&framebuffer_create_info, None)
            .map_err(|e| format!("create framebuffer: {:?}", e))?;
        framebuffers.push(framebuffer);
    }

    Ok(framebuffers)
}

unsafe fn create_depth_image(device: &Device, extent: vk::Extent3D) -> Result<Image> {
    let create_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::D16_UNORM)
        .extent(extent)
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let image = Image::new(
        device,
        device.memory_properties(),
        *create_info,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
    .map_err(|e| format!("create image: {:?}", e))?;

    Ok(image)
}

unsafe fn create_depth_image_view(
    device: &Device,
    image: &vk::Image,
    image_format: &vk::Format,
) -> Result<vk::ImageView> {
    let subresource_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::DEPTH)
        .level_count(1)
        .layer_count(1);
    let create_image_view_info = vk::ImageViewCreateInfo::builder()
        .subresource_range(*subresource_range)
        .image(*image)
        .format(*image_format)
        .view_type(vk::ImageViewType::TYPE_2D);

    let image_view = device
        .create_image_view(&create_image_view_info, None)
        .unwrap();

    Ok(image_view)
}
