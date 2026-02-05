use crate::engine::render::pipeline::error::NeuclidioRenderPipelineError;
use crate::engine::render::windowing::swap_chain::SwapChain;
use crate::error::NeuclidioResult;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::vk::{DeviceV1_0, HasBuilder, Queue};
use vulkanalia::{Device, vk};

pub mod error;

pub(crate) mod standard;
pub(crate) mod synchronization;

pub trait NeuclidioRenderPipeline {
    fn render(
        &mut self,
        logical_device: &Device,
        swap_chain: &SwapChain,
        graphics_queue: Queue,
        present_queue: Queue,
    ) -> NeuclidioResult<()>;
    fn recreate_frame_buffers(
        &mut self,
        logical_device: &Device,
        swap_chain: &SwapChain,
    ) -> NeuclidioResult<()>;
    fn destroy_frame_buffers(&self, logical_device: &Device, skip_freeing_command_buffers: bool);
    fn destroy(&self, logical_device: &Device);
}

pub(crate) fn create_shader_module(
    logical_device: &Device,
    bytecode: &[u8],
) -> NeuclidioResult<vk::ShaderModule> {
    let bytecode = Bytecode::new(bytecode).map_err(NeuclidioRenderPipelineError::ByteCodeError)?;
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
