use crate::engine::render::pipeline::common::push_constant::PushConstant;
use crate::engine::render::pipeline::common::push_constant::model::ModelPushConstant;
use crate::entity::transform::TransformExt;
use glam::{Mat4, Quat, Vec3};

#[derive(Clone)]
pub struct EuclideanTransform {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl EuclideanTransform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn rotation(&self) -> Quat {
        self.rotation
    }

    pub fn scale(&self) -> Vec3 {
        self.scale
    }

    pub fn set_position(&mut self, position: Vec3) -> &mut Self {
        self.position = position;
        self
    }

    pub fn set_rotation(&mut self, rotation: Quat) -> &mut Self {
        self.rotation = rotation;
        self
    }

    pub fn set_scale(&mut self, scale: Vec3) -> &mut Self {
        self.scale = scale;
        self
    }

    pub fn with_position(&self, position: Vec3) -> Self {
        let mut transform = self.clone();
        transform.set_position(position);
        transform
    }

    pub fn with_rotation(&self, rotation: Quat) -> Self {
        let mut transform = self.clone();
        transform.set_rotation(rotation);
        transform
    }

    pub fn with_scale(&self, scale: Vec3) -> Self {
        let mut transform = self.clone();
        transform.set_scale(scale);
        transform
    }

    fn get_model_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

impl TransformExt for EuclideanTransform {
    fn as_push_constant(&self) -> PushConstant {
        ModelPushConstant::new(self.get_model_matrix()).into()
    }
}

impl Default for EuclideanTransform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}
