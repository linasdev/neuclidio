use crate::engine::render::pipeline::common::push_constant::PushConstant;
use crate::entity::transform::euclidean::EuclideanTransform;

pub mod euclidean;

pub trait TransformExt {
    fn as_push_constant(&self) -> PushConstant;
}

pub enum Transform {
    Euclidean(EuclideanTransform),
}

impl TransformExt for Transform {
    fn as_push_constant(&self) -> PushConstant {
        match self {
            Transform::Euclidean(transform) => transform.as_push_constant(),
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::Euclidean(EuclideanTransform::default())
    }
}
