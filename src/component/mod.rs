use crate::component::camera::Camera;
use crate::component::camera::orthographic::OrthographicCamera;
use crate::component::camera::perspective::PerspectiveCamera;
use crate::component::renderable::Renderable;
use crate::component::renderable::mesh::Mesh;
use crate::id::ComponentId;
use delegate::delegate;
use derive_more::{From, TryInto};

pub mod camera;
pub mod error;
pub mod renderable;

pub trait ComponentExt: Into<Component> {
    fn id(&self) -> ComponentId;
}

#[derive(From, TryInto)]
#[try_into(ref, ref_mut)]
pub enum Component {
    #[from(Renderable, Mesh)]
    Renderable(Renderable),

    #[from(Camera, PerspectiveCamera, OrthographicCamera)]
    Camera(Camera),
}

impl ComponentExt for Component {
    delegate! {
        to match self {
            Component::Renderable(c) => c,
            Component::Camera(c) => c,
        } {
            fn id(&self) -> ComponentId;
        }
    }
}
