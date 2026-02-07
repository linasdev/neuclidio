use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::vk::{DeviceV1_0, HasBuilder};
use vulkanalia::{Device, vk};
use crate::engine::render::pipeline::standard::StandardRenderPipeline;

pub mod error;

pub(crate) mod standard;
pub(crate) mod synchronization;
pub(crate) mod uniform;
pub(crate) mod vertex;

pub trait RenderPipelineExt {
    fn render(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()>;
    fn prepare_for_reset(&self, neuclidio_window: &NeuclidioWindow);
    fn reset(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()>;
    fn destroy(self, neuclidio_window: &NeuclidioWindow);
}

pub enum RenderPipeline {
    Standard(Box<StandardRenderPipeline>),
}

impl RenderPipelineExt for RenderPipeline {
    fn render(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()> {
        match self {
            RenderPipeline::Standard(pipeline) => pipeline.render(neuclidio_window),
        }
    }

    fn prepare_for_reset(&self, neuclidio_window: &NeuclidioWindow) {
        match self {
            RenderPipeline::Standard(pipeline) => pipeline.prepare_for_reset(neuclidio_window),
        }
    }

    fn reset(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()> {
        match self {
            RenderPipeline::Standard(pipeline) => pipeline.reset(neuclidio_window),
        }
    }

    fn destroy(self, neuclidio_window: &NeuclidioWindow) {
        match self {
            RenderPipeline::Standard(pipeline) => pipeline.destroy(neuclidio_window),
        }
    }
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
