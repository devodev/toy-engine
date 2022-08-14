use std::{error::Error, ops::Deref, result};

use ash::vk;

use super::image::Image;

type Result<T> = result::Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Sampler {
    handle: vk::Sampler,

    destroyed: bool,
}

impl Sampler {
    pub(crate) unsafe fn new(
        device: &ash::Device,
        create_info: vk::SamplerCreateInfo,
    ) -> Result<Self> {
        let sampler = device.create_sampler(&create_info, None).unwrap();

        Ok(Self {
            handle: sampler,
            destroyed: false,
        })
    }

    pub(crate) unsafe fn basic(device: &ash::Device) -> Result<Self> {
        let create_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(false)
            .max_anisotropy(1.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(1.0);
        Self::new(device, *create_info)
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        if self.destroyed {
            panic!("sampler already destroyed")
        }
        device.destroy_sampler(self.handle, None);
        self.destroyed = true;
    }
}

impl Deref for Sampler {
    type Target = vk::Sampler;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Texture {
    image: Image,
    image_view: vk::ImageView,
    sampler: Sampler,

    destroyed: bool,
}

impl Texture {
    pub(crate) unsafe fn new(device: &ash::Device, image: Image, sampler: Sampler) -> Result<Self> {
        let image_view = image.create_view(
            device,
            vk::ImageViewType::TYPE_2D,
            vk::ImageAspectFlags::COLOR,
        )?;
        Ok(Self {
            image,
            image_view,
            sampler,
            destroyed: false,
        })
    }

    pub(crate) unsafe fn from_image(device: &ash::Device, image: Image) -> Result<Self> {
        let sampler = Sampler::basic(device)?;
        Self::new(device, image, sampler)
    }

    pub(crate) fn image_view(&self) -> &vk::ImageView {
        &self.image_view
    }

    pub(crate) fn sampler(&self) -> &Sampler {
        &self.sampler
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        if self.destroyed {
            panic!("texture already destroyed")
        }

        self.sampler.destroy(device);
        device.destroy_image_view(self.image_view, None);
        self.image.destroy(device);

        self.destroyed = true;
    }
}
