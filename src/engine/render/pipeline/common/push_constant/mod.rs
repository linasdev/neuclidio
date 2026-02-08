use crate::engine::render::pipeline::common::push_constant::model::ModelPushConstant;

pub mod model;

pub trait PushConstantExt: Into<PushConstant> {
    fn as_bytes(&self) -> &[u8];
}

pub enum PushConstant {
    Model(ModelPushConstant),
}

impl PushConstantExt for PushConstant {
    fn as_bytes(&self) -> &[u8] {
        match self {
            PushConstant::Model(model) => model.as_bytes(),
        }
    }
}
