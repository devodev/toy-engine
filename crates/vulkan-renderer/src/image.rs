use std::mem::{self, align_of};
use std::ops::Deref;

use ash::util::Align;
use ash::vk;

use super::buffer::Buffer;
use super::device::Device;
use super::find_memorytype_index;
use super::renderer::{copy_buffer_to_image, transition_image_layout};
use crate::Result;

#[derive(Clone, Copy, Debug)]
pub struct Image {
    create_info: vk::ImageCreateInfo,
    handle: vk::Image,

    memory: vk::DeviceMemory,
    #[allow(unused)]
    memory_requirements: vk::MemoryRequirements,

    destroyed: bool,
}

impl Image {
    pub unsafe fn new(
        device: &ash::Device,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        create_info: vk::ImageCreateInfo,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<Self> {
        let image = device
            .create_image(&create_info, None)
            .map_err(|e| format!("create image: {:?}", e))?;

        // allocate memory for the image
        let image_memory_req = device.get_image_memory_requirements(image);
        let image_memory_index =
            find_memorytype_index(&image_memory_req, device_memory_properties, properties)
                .ok_or("unable to find suitable memorytype for the image")?;
        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: image_memory_req.size,
            memory_type_index: image_memory_index,
            ..Default::default()
        };
        let image_memory = device
            .allocate_memory(&allocate_info, None)
            .map_err(|e| format!("allocate image memory: {:?}", e))?;
        device
            .bind_image_memory(image, image_memory, 0)
            .map_err(|e| format!("bind image memory: {:?}", e))?;

        Ok(Self {
            create_info,
            handle: image,
            memory: image_memory,
            memory_requirements: image_memory_req,
            destroyed: false,
        })
    }

    pub unsafe fn upload_gpu<T: Copy>(
        &mut self,
        device: &Device,
        command_pool: vk::CommandPool,
        data: &[T],
    ) -> Result<()> {
        let mut staging_buffer = {
            let staging_buffer_size = mem::size_of_val(data) as u64;
            let mut staging_buffer = Buffer::new(
                device,
                device.memory_properties(),
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                staging_buffer_size,
            )
            .map_err(|e| format!("create staging buffer: {:?}", e))?;
            staging_buffer
                .update(device, data)
                .map_err(|e| format!("update staging buffer: {:?}", e))?;
            staging_buffer
        };

        transition_image_layout(
            device,
            command_pool,
            *self.image(),
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )
        .map_err(|e| format!("transition image layout: {:?}", e))?;

        copy_buffer_to_image(
            device,
            command_pool,
            *staging_buffer,
            *self.image(),
            self.width(),
            self.height(),
        )
        .map_err(|e| format!("copy buffer to image: {:?}", e))?;

        transition_image_layout(
            device,
            command_pool,
            *self.image(),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )
        .map_err(|e| format!("transition image layout: {:?}", e))?;

        device.device_wait_idle().expect("device wait idle");
        staging_buffer.destroy(device);

        Ok(())
    }

    #[allow(unused)]
    pub unsafe fn update<T: Copy>(&mut self, device: &ash::Device, data: &[T]) -> Result<()> {
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

    pub fn image(&self) -> &vk::Image {
        &self.handle
    }

    pub fn format(&self) -> &vk::Format {
        &self.create_info.format
    }

    pub fn width(&self) -> u32 {
        self.create_info.extent.width
    }

    pub fn height(&self) -> u32 {
        self.create_info.extent.height
    }

    pub unsafe fn create_view(
        &self,
        device: &ash::Device,
        view_type: vk::ImageViewType,
        aspect_mask: vk::ImageAspectFlags,
    ) -> Result<vk::ImageView> {
        let image_view_info = vk::ImageViewCreateInfo {
            view_type,
            format: *self.format(),
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            },
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            image: *self.image(),
            ..Default::default()
        };
        let image_view = device
            .create_image_view(&image_view_info, None)
            .map_err(|e| format!("create image view: {:?}", e))?;

        Ok(image_view)
    }

    pub unsafe fn destroy(&mut self, device: &ash::Device) {
        if self.destroyed {
            panic!("image already destroyed")
        }
        device.free_memory(self.memory, None);
        device.destroy_image(self.handle, None);
        self.destroyed = true;
    }
}

impl Deref for Image {
    type Target = vk::Image;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
