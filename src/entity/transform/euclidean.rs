use crate::engine::render::pipeline::uniform::ModelViewProjection;
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
    fn load_into_uniform_buffer(&self, uniform_buffer_memory: *mut u8) {
        let mvp_memory_object = unsafe {
            let mvp_memory: *mut ModelViewProjection = uniform_buffer_memory.cast();
            mvp_memory.as_mut()
        };

        let mvp = ModelViewProjection::new(
            self.get_model_matrix(),
            Mat4::IDENTITY,
            Mat4::perspective_rh(75f32.to_radians(), (1000 as f32) / (750 as f32), 0.1, 100.0),
        );

        if let Some(mvp_memory_object) = mvp_memory_object {
            *mvp_memory_object = mvp;
        }
    }

    fn size_in_uniform_buffer(&self) -> u32 {
        ModelViewProjection::size_in_uniform_buffer()
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
