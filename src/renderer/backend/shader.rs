use std::io;
use std::ops::Deref;

use ash::util::read_spv;
use ash::vk;

use crate::Result;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct Shader {
    /// Shader modules contain shader code and one or more entry points. Shaders
    /// are selected from a shader module by specifying an entry point as part
    /// of pipeline creation. The stages of a pipeline can use shaders that come
    /// from different modules.
    ///
    /// NOTE: The shader code defining a shader module must be
    /// in the SPIR-V format, as described by the Vulkan Environment for SPIR-V
    /// appendix.
    pub(crate) handle: vk::ShaderModule,

    destroyed: bool,
}

impl Shader {
    pub(crate) unsafe fn new<R>(device: &ash::Device, cursor: &mut R) -> Result<Self>
    where
        R: io::Read + io::Seek,
    {
        let code = read_spv(cursor)
            .map_err(|e| format!("failed to read shader spv from cursor: {:?}", e))?;
        let shader_info = vk::ShaderModuleCreateInfo::builder().code(&code);

        let shader_module = device
            .create_shader_module(&shader_info, None)
            .map_err(|e| format!("shader module error: {:?}", e))?;

        Ok(Self {
            handle: shader_module,
            destroyed: false,
        })
    }

    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        if self.destroyed {
            panic!("shader already destroyed")
        }
        device.destroy_shader_module(self.handle, None);
        self.destroyed = true;
    }
}

impl Deref for Shader {
    type Target = vk::ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
