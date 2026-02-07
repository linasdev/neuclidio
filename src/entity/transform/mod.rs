use crate::entity::transform::euclidean::EuclideanTransform;

pub mod euclidean;

pub trait TransformExt {
    fn load_into_uniform_buffer(&self, uniform_buffer_memory: *mut u8);
    fn size_in_uniform_buffer(&self) -> u32;
}

pub enum Transform {
    Euclidean(EuclideanTransform),
}

impl TransformExt for Transform {
    fn load_into_uniform_buffer(&self, uniform_buffer_memory: *mut u8) {
        match self {
            Transform::Euclidean(transform) => {
                transform.load_into_uniform_buffer(uniform_buffer_memory)
            }
        }
    }

    fn size_in_uniform_buffer(&self) -> u32 {
        match self {
            Transform::Euclidean(transform) => transform.size_in_uniform_buffer(),
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::Euclidean(EuclideanTransform::default())
    }
}
