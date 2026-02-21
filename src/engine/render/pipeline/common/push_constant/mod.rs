use crate::engine::render::pipeline::common::push_constant::model::ModelPushConstant;
use delegate::delegate;

pub mod model;

pub trait PushConstantExt: Into<PushConstant> {
    fn as_bytes(&self) -> &[u8];
}

pub enum PushConstant {
    Model(ModelPushConstant),
}

impl PushConstantExt for PushConstant {
    delegate! {
        to match self {
            PushConstant::Model(c) => c,
        } {
            fn as_bytes(&self) -> &[u8];
        }
    }
}
