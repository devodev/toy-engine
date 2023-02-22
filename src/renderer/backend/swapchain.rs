use ash::extensions::khr;
use ash::vk;

use super::device::Device;
use crate::Result;

pub(crate) struct Swapchain {
    /// A swapchain object (a.k.a. swapchain) provides the ability to present
    /// rendering results to a surface.
    swapchain: vk::SwapchainKHR,
    swapchain_loader: khr::Swapchain,

    /// The image format of the surface.
    image_format: vk::Format,

    /// Image objects are not directly accessed by pipeline shaders for reading
    /// or writing image data. Instead, image views representing contiguous
    /// ranges of the image subresources and containing additional metadata are
    /// used for that purpose.
    present_image_views: Vec<vk::ImageView>,

    /// The image index returned by a call to acquire_next_image.
    current_image_index: usize,
}

impl Swapchain {
    pub(crate) unsafe fn new(device: &Device, window_extent: vk::Extent2D) -> Result<Self> {
        // create swapchain
        let (swapchain, swapchain_loader, images, image_format) =
            create_swapchain(device, window_extent)
                .map_err(|e| format!("create swapchain: {:?}", e))?;

        // create image views used for writing image data by shaders
        let present_image_views = create_present_image_views(device, &images, image_format)
            .map_err(|e| format!("create present image views from swapchain: {:?}", e))?;

        Ok(Self {
            swapchain,
            swapchain_loader,
            image_format,
            present_image_views,
            current_image_index: 0,
        })
    }

    pub(crate) fn current_index(&self) -> usize {
        self.current_image_index
    }

    pub(crate) fn image_format(&self) -> &vk::Format {
        &self.image_format
    }

    pub(crate) fn image_views(&self) -> &[vk::ImageView] {
        &self.present_image_views
    }

    pub(crate) unsafe fn acquire_next_image(
        &mut self,
        timeout: u64,
        semaphore: &vk::Semaphore,
        fence: &vk::Fence,
    ) -> Result<bool> {
        let suboptimal = match self.swapchain_loader.acquire_next_image(
            self.swapchain,
            timeout,
            *semaphore,
            *fence,
        ) {
            Ok((idx, suboptimal)) => {
                self.current_image_index = idx as usize;
                suboptimal
            }
            Err(e) => {
                if e != vk::Result::ERROR_OUT_OF_DATE_KHR {
                    return Err(format!("acquire image: {:?}", e).into());
                }
                true
            }
        };

        Ok(suboptimal)
    }

    /// wait_sempahores specifies the semaphores to wait for before issuing the
    /// present request
    pub(crate) unsafe fn queue_present(
        &mut self,
        device: &Device,
        wait_sempahores: &[vk::Semaphore],
    ) -> Result<bool> {
        // queue image for presentation
        let swapchains = [self.swapchain];
        let image_indices = [self.current_image_index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(wait_sempahores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        let suboptimal = match self
            .swapchain_loader
            .queue_present(*device.graphics_queue(), &present_info)
        {
            Ok(suboptimal) => suboptimal,
            Err(e) => match e {
                vk::Result::ERROR_OUT_OF_DATE_KHR => true,
                err => return Err(format!("queue present: {:?}", err).into()),
            },
        };

        Ok(suboptimal)
    }

    // Make sure to call device.device_wait_idle() prior to calling destroy.
    pub(crate) unsafe fn destroy(&mut self, device: &Device) {
        // image views
        for image_view in self.present_image_views.drain(..) {
            device.destroy_image_view(image_view, None);
        }
        // swapchain
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None);
    }
}

unsafe fn create_swapchain(
    device: &Device,
    window_extent: vk::Extent2D,
) -> Result<(vk::SwapchainKHR, khr::Swapchain, Vec<vk::Image>, vk::Format)> {
    // Obtain swapchain support details from the device
    let swapchain_support = device
        .swapchain_support_details()
        .map_err(|e| format!("obtain swapchain support details: {:?}", e))?;

    // Select swapchain attributes
    let surface_format = select_surface_format(&swapchain_support.formats);
    let image_count = select_image_count(swapchain_support.capabilities);
    let pre_transform = select_pre_transform(swapchain_support.capabilities);
    let extent = select_extent(swapchain_support.capabilities, window_extent);
    let present_mode = select_present_mode(&swapchain_support.present_modes);

    // create swapchain
    let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(*device.surface())
        .min_image_count(image_count)
        .image_color_space(surface_format.color_space)
        .image_format(surface_format.format)
        .image_extent(extent)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(pre_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .image_array_layers(1);
    let swapchain_loader = khr::Swapchain::new(device.instance(), device);
    let swapchain = swapchain_loader
        .create_swapchain(&swapchain_create_info, None)
        .map_err(|e| format!("create swapchain: {:?}", e))?;

    // obtain swapchain images
    let images = swapchain_loader
        .get_swapchain_images(swapchain)
        .map_err(|e| format!("obtain swapchain images: {:?}", e))?;

    Ok((swapchain, swapchain_loader, images, surface_format.format))
}

// Select optimal surface format. If not found, fallback to the first format
// available.
fn select_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    formats
        .iter()
        .cloned()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or(formats[0])
}

// Select the minimum image count supported +1. If greater than maximum, clamp
// to max.
fn select_image_count(capabilities: vk::SurfaceCapabilitiesKHR) -> u32 {
    let mut desired_image_count = capabilities.min_image_count + 1;
    if capabilities.max_image_count > 0 && desired_image_count > capabilities.max_image_count {
        desired_image_count = capabilities.max_image_count;
    }
    desired_image_count
}

// Select a transform that supports IDENTITY. If not available, fallback to
// the current transform.
fn select_pre_transform(capabilities: vk::SurfaceCapabilitiesKHR) -> vk::SurfaceTransformFlagsKHR {
    if capabilities
        .supported_transforms
        .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
    {
        // IDENTITY pre_transform specifies that image content is presented without
        // being transformed.
        vk::SurfaceTransformFlagsKHR::IDENTITY
    } else {
        capabilities.current_transform
    }
}

// Select the optimal extent based on the provided capabilities.
fn select_extent(
    capabilities: vk::SurfaceCapabilitiesKHR,
    window_extent: vk::Extent2D,
) -> vk::Extent2D {
    if capabilities.current_extent.width != std::u32::MAX {
        return capabilities.current_extent;
    }

    let mut current_extent = window_extent;
    current_extent.width = std::cmp::max(
        capabilities.min_image_extent.width,
        std::cmp::min(capabilities.max_image_extent.width, current_extent.width),
    );
    current_extent.height = std::cmp::max(
        capabilities.min_image_extent.height,
        std::cmp::min(capabilities.max_image_extent.height, current_extent.height),
    );
    current_extent
}

// Select MAILBOX present mode. If not available, fallback to FIFO.
fn select_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    present_modes
        .iter()
        .cloned()
        .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

unsafe fn create_present_image_views(
    device: &Device,
    images: &[vk::Image],
    image_format: vk::Format,
) -> Result<Vec<vk::ImageView>> {
    let mut image_views: Vec<vk::ImageView> = Vec::new();
    for create_view_info in images.iter().map(|&image| {
        vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(image_format)
            .components(
                *vk::ComponentMapping::builder()
                    .r(vk::ComponentSwizzle::R)
                    .g(vk::ComponentSwizzle::G)
                    .b(vk::ComponentSwizzle::B)
                    .a(vk::ComponentSwizzle::A),
            )
            .subresource_range(
                *vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1),
            )
            .image(image)
    }) {
        let image_view = device
            .create_image_view(&create_view_info, None)
            .map_err(|e| format!("create image view: {:?}", e))?;
        image_views.push(image_view);
    }

    Ok(image_views)
}
