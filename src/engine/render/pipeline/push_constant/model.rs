use crate::engine::render::pipeline::push_constant::{PushConstant, PushConstantExt};
use glam::Mat4;
use std::slice;
use vulkanalia::vk;
use vulkanalia::vk::HasBuilder;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ModelPushConstant {
    model: Mat4,
}

impl ModelPushConstant {
    pub fn new(model: Mat4) -> Self {
        Self { model }
    }

    pub fn push_constant_range() -> vk::PushConstantRange {
        vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(Self::size_in_push_constant())
            .build()
    }

    pub fn size_in_push_constant() -> u32 {
        size_of::<Self>() as u32
    }
}

impl PushConstantExt for ModelPushConstant {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self as *const Self as *const u8,
                Self::size_in_push_constant() as usize,
            )
        }
    }
}

impl From<ModelPushConstant> for PushConstant {
    fn from(value: ModelPushConstant) -> Self {
        Self::Model(value)
    }
}
