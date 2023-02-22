#![allow(clippy::missing_safety_doc)]

/// Vulkan backend package.
pub mod buffer;
pub mod descriptor;
pub mod device;
pub mod image;
pub mod pipeline;
pub mod renderer;
pub mod renderpass;
pub mod shader;
pub mod swapchain;
pub mod texture;

use std::{error, result};

use ash::vk;

type Result<T> = result::Result<T, Box<dyn error::Error>>;

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
