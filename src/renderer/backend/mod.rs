use ash::vk;

/// Vulkan backend package.
pub(crate) mod buffer;
pub(crate) mod descriptor;
pub(crate) mod device;
pub(crate) mod image;
pub(crate) mod pipeline;
pub(crate) mod renderer;
pub(crate) mod renderpass;
pub(crate) mod shader;
pub(crate) mod swapchain;
pub(crate) mod texture;

fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}
