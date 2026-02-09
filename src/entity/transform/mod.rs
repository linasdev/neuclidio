use crate::engine::render::pipeline::common::push_constant::PushConstant;
use crate::entity::transform::euclidean::EuclideanTransform;

pub mod euclidean;

pub trait TransformExt: Into<Transform> {
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

#[allow(irrefutable_let_patterns)]
impl<'a> TryFrom<&'a mut Transform> for &'a mut EuclideanTransform {
    type Error = ();

    fn try_from(transform: &'a mut Transform) -> Result<Self, Self::Error> {
        if let Transform::Euclidean(transform) = transform {
            return Ok(transform);
        }

        Err(())
    }
}
