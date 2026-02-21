use crate::component::ComponentExt;
use crate::component::camera::orthographic::OrthographicCamera;
use crate::component::camera::perspective::PerspectiveCamera;
use crate::id::ComponentId;
use delegate::delegate;
use derive_more::From;
use glam::Mat4;

pub mod orthographic;
pub mod perspective;

pub trait CameraExt: Into<Camera> + Clone {
    fn get_view_matrix(&self) -> Mat4;
    fn get_projection_matrix(&mut self, aspect_ratio: f32) -> Mat4;
}

#[derive(From, Clone)]
pub enum Camera {
    Perspective(PerspectiveCamera),
    Orthographic(OrthographicCamera),
}

impl CameraExt for Camera {
    delegate! {
        to match self {
            Camera::Perspective(c) => c,
            Camera::Orthographic(c) => c,
        } {
            fn get_view_matrix(&self) -> Mat4;
            fn get_projection_matrix(&mut self, aspect_ratio: f32) -> Mat4;
        }
    }
}

impl ComponentExt for Camera {
    delegate! {
        to match self {
            Camera::Perspective(c) => c,
            Camera::Orthographic(c) => c,
        } {
            fn id(&self) -> ComponentId;
        }
    }
}
