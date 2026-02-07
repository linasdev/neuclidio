use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::vk::{DeviceV1_0, HasBuilder};
use vulkanalia::{Device, vk};

pub mod error;

pub(crate) mod standard;
pub(crate) mod synchronization;
pub(crate) mod uniform;
pub(crate) mod vertex;

pub trait RenderPipeline {
    fn render(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()>;
    fn prepare_for_reset(&self, neuclidio_window: &NeuclidioWindow);
    fn reset(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()>;
    fn destroy(self: Box<Self>, neuclidio_window: &NeuclidioWindow);
}

pub(crate) fn create_shader_module(
    logical_device: &Device,
    bytecode: &[u8],
) -> NeuclidioResult<vk::ShaderModule> {
    let bytecode = Bytecode::new(bytecode).map_err(RenderPipelineError::from)?;
    let shader_module_create_info = vk::ShaderModuleCreateInfo::builder()
        .code(bytecode.code())
        .code_size(bytecode.code_size())
        .build();

    let shader_module =
        unsafe { logical_device.create_shader_module(&shader_module_create_info, None)? };

    Ok(shader_module)
}

pub(crate) fn destroy_shader_module(logical_device: &Device, shader_module: vk::ShaderModule) {
    unsafe {
        logical_device.destroy_shader_module(shader_module, None);
    }
}
