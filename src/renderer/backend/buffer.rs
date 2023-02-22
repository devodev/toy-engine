use std::{mem::align_of, ops::Deref};

use ash::{util::Align, vk};

use super::find_memorytype_index;
use crate::Result;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Buffer {
    handle: vk::Buffer,

    memory: vk::DeviceMemory,
    memory_requirements: vk::MemoryRequirements,

    destroyed: bool,
}

impl Buffer {
    pub(crate) unsafe fn new(
        device: &ash::Device,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
        size: u64,
    ) -> Result<Self> {
        // Create buffer object
        let buffer_info = vk::BufferCreateInfo {
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let buffer = device
            .create_buffer(&buffer_info, None)
            .map_err(|e| format!("create buffer: {:?}", e))?;

        // allocate memory for the buffer
        let buffer_memory_req = device.get_buffer_memory_requirements(buffer);
        let buffer_memory_index =
            find_memorytype_index(&buffer_memory_req, device_memory_properties, properties)
                .ok_or("unable to find suitable memorytype for the buffer")?;
        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: buffer_memory_req.size,
            memory_type_index: buffer_memory_index,
            ..Default::default()
        };
        let buffer_memory = device
            .allocate_memory(&allocate_info, None)
            .map_err(|e| format!("allocate buffer memory: {:?}", e))?;
        device
            .bind_buffer_memory(buffer, buffer_memory, 0)
            .map_err(|e| format!("bind buffer memory: {:?}", e))?;

        Ok(Self {
            handle: buffer,
            memory: buffer_memory,
            memory_requirements: buffer_memory_req,
            destroyed: false,
        })
    }

    pub(crate) fn buffer(&self) -> &vk::Buffer {
        &self.handle
    }

    pub(crate) unsafe fn update<T: Copy>(
        &mut self,
        device: &ash::Device,
        data: &[T],
    ) -> Result<()> {
        // obtain pointer into data
        let buffer_ptr: *mut std::os::raw::c_void = device
            .map_memory(
                self.memory,
                0,
                self.memory_requirements.size,
                vk::MemoryMapFlags::empty(),
            )
            .map_err(|e| format!("map buffer memory: {:?}", e))?;
        let mut slice = Align::new(
            buffer_ptr,
            align_of::<T>() as u64,
            self.memory_requirements.size,
        );

        // copy data into buffer
        slice.copy_from_slice(data);
        device.unmap_memory(self.memory);

        Ok(())
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        if self.destroyed {
            panic!("buffer already destroyed")
        }
        device.free_memory(self.memory, None);
        device.destroy_buffer(self.handle, None);
        self.destroyed = true;
    }
}

impl Deref for Buffer {
    type Target = vk::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
