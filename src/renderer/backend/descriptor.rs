use std::ops::Deref;

use ash::vk;

use super::buffer::Buffer;
use super::texture::Texture;
use crate::Result;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct DescriptorPool {
    /// A descriptor pool maintains a pool of descriptors, from which descriptor
    /// sets are allocated.
    ///
    /// NOTE: Descriptor pools are externally synchronized,
    /// meaning that the application must not allocate and/or free descriptor
    /// sets from the same pool in multiple threads simultaneously.
    pub(crate) handle: vk::DescriptorPool,

    destroyed: bool,
}

impl DescriptorPool {
    pub(crate) unsafe fn new(
        device: &ash::Device,
        sizes: &[vk::DescriptorPoolSize],
        max_count: u32,
    ) -> Result<Self> {
        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(sizes)
            .max_sets(max_count);
        let descriptor_pool = device
            .create_descriptor_pool(&descriptor_pool_info, None)
            .map_err(|e| format!("create descriptor pool: {:?}", e))?;

        Ok(Self {
            handle: descriptor_pool,
            destroyed: false,
        })
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        if self.destroyed {
            panic!("descriptor pool already destroyed")
        }
        device.destroy_descriptor_pool(self.handle, None);
        self.destroyed = true;
    }
}

impl Deref for DescriptorPool {
    type Target = vk::DescriptorPool;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct DescriptorSetLayout {
    /// A descriptor set layout object is defined by an array of zero or more
    /// descriptor bindings. Each individual descriptor binding is specified by
    /// a descriptor type, a count (array size) of the number of descriptors in
    /// the binding, a set of shader stages that can access the binding, and (if
    /// using immutable samplers) an array of sampler descriptors.
    pub(crate) handle: vk::DescriptorSetLayout,

    destroyed: bool,
}

impl DescriptorSetLayout {
    pub(crate) unsafe fn new(
        device: &ash::Device,
        bindings: &[vk::DescriptorSetLayoutBinding],
    ) -> Result<Self> {
        let descriptor_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);
        let descriptor_set_layout = device
            .create_descriptor_set_layout(&descriptor_info, None)
            .map_err(|e| format!("create descriptor set layout: {:?}", e))?;

        Ok(Self {
            handle: descriptor_set_layout,
            destroyed: false,
        })
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        if self.destroyed {
            panic!("descriptor set layout already destroyed")
        }
        device.destroy_descriptor_set_layout(self.handle, None);
        self.destroyed = true;
    }
}

impl Deref for DescriptorSetLayout {
    type Target = vk::DescriptorSetLayout;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct DescriptorSet {
    pub(crate) handle: vk::DescriptorSet,
}

impl DescriptorSet {
    pub(crate) unsafe fn new(
        device: &ash::Device,
        pool: &DescriptorPool,
        layouts: &[DescriptorSetLayout],
    ) -> Result<Vec<Self>> {
        let layouts = layouts.iter().map(|d| d.handle).collect::<Vec<_>>();
        let desc_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool.handle)
            .set_layouts(&layouts);
        let descriptor_sets = device
            .allocate_descriptor_sets(&desc_alloc_info)
            .map_err(|e| format!("allocate descriptor sets: {:?}", e))?;

        let descriptor_sets = descriptor_sets
            .iter()
            .map(|d| Self { handle: *d })
            .collect::<Vec<_>>();
        Ok(descriptor_sets)
    }

    pub(crate) unsafe fn update(
        &self,
        device: &ash::Device,
        descriptor_writes: &[vk::WriteDescriptorSet],
    ) -> Result<()> {
        device.update_descriptor_sets(descriptor_writes, &[]);

        Ok(())
    }

    pub(crate) unsafe fn update_ubo(
        &self,
        device: &ash::Device,
        buffer: &Buffer,
        buffer_offset: u64,
        // size in bytes that is used for this descriptor update, or VK_WHOLE_SIZE to use the range
        // from offset to the end of the buffer.
        buffer_size: u64, // mem::size_of_val(&buffer_data) as u64
    ) -> Result<()> {
        let descriptor_set_info = vk::DescriptorBufferInfo {
            buffer: *buffer.buffer(),
            range: buffer_size,
            offset: buffer_offset,
        };
        let write_desc_sets = [vk::WriteDescriptorSet {
            dst_set: self.handle,
            descriptor_count: 1,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            p_buffer_info: &descriptor_set_info,
            ..Default::default()
        }];
        self.update(device, &write_desc_sets)
    }

    #[allow(unused)]
    pub(crate) unsafe fn update_texture(
        &self,
        device: &ash::Device,
        texture: &Texture,
    ) -> Result<()> {
        let descriptor_set_info = vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: *texture.image_view(),
            sampler: **texture.sampler(),
        };
        let write_desc_sets = [vk::WriteDescriptorSet {
            dst_set: self.handle,
            dst_binding: 1,
            descriptor_count: 1,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            p_image_info: &descriptor_set_info,
            ..Default::default()
        }];
        self.update(device, &write_desc_sets)
    }
}

impl Deref for DescriptorSet {
    type Target = vk::DescriptorSet;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
