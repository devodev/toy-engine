use std::{borrow::Cow, error::Error, ffi::CStr, ops::Deref, os::raw::c_char, result};

use ash::{
    extensions::{ext, khr},
    vk::{self, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessengerEXT},
    Entry,
};
use log::{debug, error, info, warn};
use winit::window::Window;

type Result<T> = result::Result<T, Box<dyn Error>>;

#[derive(Debug, Default)]
pub(crate) struct SwapChainSupportDetails {
    /// Structure describing a supported swapchain format-color space pair.
    pub(crate) formats: Vec<vk::SurfaceFormatKHR>,
    /// Structure describing capabilities of a surface.
    pub(crate) capabilities: vk::SurfaceCapabilitiesKHR,
    /// Presentation mode supported for a surface.
    pub(crate) present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapChainSupportDetails {
    pub(crate) fn new(
        formats: Vec<vk::SurfaceFormatKHR>,
        capabilities: vk::SurfaceCapabilitiesKHR,
        present_modes: Vec<vk::PresentModeKHR>,
    ) -> Self {
        Self {
            formats,
            capabilities,
            present_modes,
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Default)]
struct ApiVersion {
    variant: u32,
    major: u32,
    minor: u32,
    patch: u32,
}

impl ApiVersion {
    const fn new(variant: u32, major: u32, minor: u32, patch: u32) -> Self {
        Self {
            variant,
            major,
            minor,
            patch,
        }
    }
}

impl From<ApiVersion> for u32 {
    fn from(val: ApiVersion) -> Self {
        vk::make_api_version(val.variant, val.major, val.minor, val.patch)
    }
}

// apiVersion must be the highest version of Vulkan that the application is
// designed to use
const API_VERSION: ApiVersion = ApiVersion::new(0, 1, 2, 0);

pub(crate) struct Device {
    /// There is no global state in Vulkan and all per-application state is
    /// stored in a VkInstance object. Creating a VkInstance object initializes
    /// the Vulkan library and allows the application to pass information about
    /// itself to the implementation.
    instance: ash::Instance,

    /// Handles Vulkan debug messages by passing them to a debug callback.
    debug_utils_loader: ext::DebugUtils,
    debug_callback: vk::DebugUtilsMessengerEXT,

    /// Native platform surface or window objects are abstracted by surface
    /// objects, which are represented by VkSurfaceKHR handles.
    surface: vk::SurfaceKHR,
    surface_loader: khr::Surface,

    /// Vulkan separates the concept of physical and logical devices. A physical
    /// device usually represents a single complete implementation of Vulkan
    /// (excluding instance-level functionality) available to the host, of which
    /// there are a finite number.
    physical_device: vk::PhysicalDevice,

    #[allow(unused)]
    /// Structure specifying physical device memory properties.
    physical_device_memory_properties: vk::PhysicalDeviceMemoryProperties,

    /// Logical devices are represented by VkDevice handles.
    handle: ash::Device,

    /// Device queue used to submit graphics command buffers.
    gfx_queue: vk::Queue,
    gfx_queue_family_index: u32,
}

impl Device {
    /// Returns a new device that allows access to the underlying physical
    /// device.
    pub(crate) unsafe fn new(app_name: impl AsRef<str>, window: &Window) -> Result<Self> {
        // Load entry points from a Vulkan loader linked at compile time.
        // NOTE: requires that the build environment have Vulkan development packages
        // installed.
        let entry = ash::Entry::linked();

        // create Vulkan instance
        let instance = create_instance(&entry, window, app_name)?;

        // setup debug callback that logs Vulkan debug messages
        let (debug_utils_loader, debug_callback) = create_debug_callback(&entry, &instance)
            .map_err(|e| format!("create Vulkan debug callback: {:?}", e))?;

        // create surface from window
        let (surface, surface_loader) = create_surface(&entry, &instance, window)
            .map_err(|e| format!("create Vulkan surface: {:?}", e))?;

        // find physical device (graphics card) that supports graphics and our window
        let (physical_device, gfx_queue_family_index) =
            find_suitable_physical_device(&instance, &surface_loader, &surface).map_err(|e| {
                format!("find suitable physical device (supports graphics): {:?}", e)
            })?;

        // get physical device memory properties
        // this is used when creating different types of buffers
        let physical_device_memory_properties =
            instance.get_physical_device_memory_properties(physical_device);

        // create logical Vulkan device handle
        let device = create_device(&instance, &physical_device, gfx_queue_family_index)
            .map_err(|e| format!("create Vulkan device: {:?}", e))?;

        // The queue handle used to submit command buffers
        // For now, use the same queue for both graphics and compute command buffers
        let gfx_queue = device.get_device_queue(gfx_queue_family_index, 0);

        Ok(Self {
            instance,
            debug_utils_loader,
            debug_callback,
            surface,
            surface_loader,
            physical_device,
            physical_device_memory_properties,
            handle: device,
            gfx_queue,
            gfx_queue_family_index,
        })
    }

    /// Returns a handle to the Vulkan instance.
    pub(crate) fn instance(&self) -> &ash::Instance {
        &self.instance
    }

    /// Returns a handle to the Vulkan surface.
    pub(crate) fn surface(&self) -> &vk::SurfaceKHR {
        &self.surface
    }

    /// Returns a handle to the graphics queue for this device.
    pub(crate) fn graphics_queue(&self) -> &vk::Queue {
        &self.gfx_queue
    }

    /// Creates a new command pool for the graphics queue.
    pub(crate) unsafe fn create_command_pool(&self) -> Result<vk::CommandPool> {
        // command buffer pool
        let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(self.gfx_queue_family_index);
        let command_pool = self
            .handle
            .create_command_pool(&command_pool_create_info, None)
            .map_err(|e| format!("create command pool: {:?}", e))?;

        Ok(command_pool)
    }

    /// Creates new command buffers from the provided command pool.
    pub(crate) unsafe fn create_command_buffers(
        &self,
        command_pool: &vk::CommandPool,
        count: u32,
    ) -> Result<Vec<vk::CommandBuffer>> {
        // allocated command buffers
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(count)
            .command_pool(*command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffers = self
            .handle
            .allocate_command_buffers(&command_buffer_allocate_info)
            .map_err(|e| format!("allocate command buffers: {:?}", e))?;

        Ok(command_buffers)
    }

    /// Returns a handle to the physical device memory properties.
    pub(crate) fn memory_properties(&self) -> &vk::PhysicalDeviceMemoryProperties {
        &self.physical_device_memory_properties
    }

    /// Returns surface attributes needed to create a swapchain for this device.
    pub(crate) unsafe fn swapchain_support_details(&self) -> Result<SwapChainSupportDetails> {
        let formats = self
            .surface_loader
            .get_physical_device_surface_formats(self.physical_device, self.surface)
            .map_err(|e| format!("obtain physical device surface formats: {:?}", e))?;
        let capabilities = self
            .surface_loader
            .get_physical_device_surface_capabilities(self.physical_device, self.surface)
            .map_err(|e| format!("obtain physical device surface capabilities: {:?}", e))?;
        let present_modes = self
            .surface_loader
            .get_physical_device_surface_present_modes(self.physical_device, self.surface)
            .map_err(|e| format!("obtain physical device surface present modes: {:?}", e))?;

        Ok(SwapChainSupportDetails::new(
            formats,
            capabilities,
            present_modes,
        ))
    }

    // Make sure to call device.device_wait_idle() prior to calling destroy.
    pub(crate) unsafe fn destroy(&self) {
        // device
        self.handle.destroy_device(None);
        // surface
        self.surface_loader.destroy_surface(self.surface, None);
        // debug callback
        self.debug_utils_loader
            .destroy_debug_utils_messenger(self.debug_callback, None);
        // instance
        self.instance.destroy_instance(None);
    }
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

unsafe fn create_instance(
    entry: &ash::Entry,
    window: &Window,
    app_name: impl AsRef<str>,
) -> Result<ash::Instance> {
    // gather required Vulkan layers
    // NOTE: Make sure we enable validation layers to catch any issue during
    // development. These can be logged by setting up a debug callback using
    // DebugUtils. Should be disabled in release mode to improve
    // performance.
    let layer_names = [CStr::from_bytes_with_nul_unchecked(
        b"VK_LAYER_KHRONOS_validation\0",
    )];
    let layers_names_raw: Vec<*const c_char> = layer_names
        .iter()
        .map(|raw_name| raw_name.as_ptr())
        .collect();

    // gather required vulkan extensions from the provided window handle
    let mut extension_names = ash_window::enumerate_required_extensions(window)
        .map_err(|e| format!("enumerate required extensions from window: {:?}", e))?
        .to_vec();
    extension_names.push(ext::DebugUtils::name().as_ptr());

    let app_name_nul_terminated = format!("{}\0", app_name.as_ref());
    let app_name_bytes = app_name_nul_terminated.as_bytes();
    let app_name = CStr::from_bytes_with_nul_unchecked(app_name_bytes);
    let appinfo = vk::ApplicationInfo::builder()
        .application_name(app_name)
        .application_version(0)
        .engine_name(app_name)
        .engine_version(0)
        .api_version(API_VERSION.into());

    let create_flags = vk::InstanceCreateFlags::default();
    let create_info = vk::InstanceCreateInfo::builder()
        .enabled_layer_names(&layers_names_raw)
        .enabled_extension_names(&extension_names)
        .application_info(&appinfo)
        .flags(create_flags);

    let instance = entry
        .create_instance(&create_info, None)
        .map_err(|e| format!("Vulkan instance creation: {:?}", e))?;

    Ok(instance)
}

unsafe fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: &Window,
) -> Result<(vk::SurfaceKHR, khr::Surface)> {
    let surface = ash_window::create_surface(entry, instance, &window, None)
        .map_err(|e| format!("create surface from window: {:?}", e))?;
    let surface_loader = khr::Surface::new(entry, instance);

    Ok((surface, surface_loader))
}

unsafe fn find_suitable_physical_device(
    instance: &ash::Instance,
    surface_loader: &khr::Surface,
    surface: &vk::SurfaceKHR,
) -> Result<(vk::PhysicalDevice, u32)> {
    let pdevices = instance
        .enumerate_physical_devices()
        .map_err(|e| format!("enumerate physical devices: {:?}", e))?;
    let (pdevice, gfx_queue_family_index) = pdevices
        .iter()
        .find_map(|pdevice| {
            instance
                .get_physical_device_queue_family_properties(*pdevice)
                .iter()
                .enumerate()
                .find_map(|(index, info)| {
                    let supports_graphic_and_surface = info
                        .queue_flags
                        .contains(vk::QueueFlags::GRAPHICS)
                        && surface_loader
                            .get_physical_device_surface_support(*pdevice, index as u32, *surface)
                            .unwrap();
                    if supports_graphic_and_surface {
                        Some((*pdevice, index))
                    } else {
                        None
                    }
                })
        })
        .ok_or("Couldn't find suitable device: {:?}")?;

    Ok((pdevice, gfx_queue_family_index as u32))
}

unsafe fn create_device(
    instance: &ash::Instance,
    physical_device: &vk::PhysicalDevice,
    queue_family_index: u32,
) -> Result<ash::Device> {
    let priorities = [1.0];
    let queue_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities);

    let device_extension_names_raw = [khr::Swapchain::name().as_ptr()];
    let features = vk::PhysicalDeviceFeatures {
        shader_clip_distance: 1,
        ..Default::default()
    };
    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(std::slice::from_ref(&queue_info))
        .enabled_extension_names(&device_extension_names_raw)
        .enabled_features(&features);

    let device: ash::Device = instance
        .create_device(*physical_device, &device_create_info, None)
        .map_err(|e| format!("create Vulkan device: {:?}", e))?;

    Ok(device)
}

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    let msg = format!(
        "[VULKAN][{:?}][{} ({})] {}",
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message
    );
    match message_severity {
        DebugUtilsMessageSeverityFlagsEXT::VERBOSE => debug!("{msg}"),
        DebugUtilsMessageSeverityFlagsEXT::INFO => info!("{msg}"),
        DebugUtilsMessageSeverityFlagsEXT::WARNING => warn!("{msg}"),
        _ => error!("{msg}"),
    }

    vk::FALSE
}

pub unsafe fn create_debug_callback(
    entry: &Entry,
    instance: &ash::Instance,
) -> Result<(ext::DebugUtils, DebugUtilsMessengerEXT)> {
    let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(debug_callback));

    let debug_utils_loader = ext::DebugUtils::new(entry, instance);
    let debug_call_back = debug_utils_loader
        .create_debug_utils_messenger(&debug_info, None)
        .map_err(|e| format!("create debug utils messenger: {:?}", e))?;

    Ok((debug_utils_loader, debug_call_back))
}
