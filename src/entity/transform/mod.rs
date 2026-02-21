use crate::engine::render::pipeline::common::push_constant::PushConstant;
use crate::entity::transform::euclidean::EuclideanTransform;
use delegate::delegate;
use derive_more::{From, TryInto};

pub mod euclidean;

pub trait TransformExt: Into<Transform> {
    fn as_push_constant(&self) -> PushConstant;
}

#[derive(From, TryInto)]
#[try_into(ref_mut)]
pub enum Transform {
    Euclidean(EuclideanTransform),
}

impl TransformExt for Transform {
    delegate! {
        to match self {
            Transform::Euclidean(t) => t,
        } {
            fn as_push_constant(&self) -> PushConstant;
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::Euclidean(EuclideanTransform::default())
    }
}
