use crate::component::mesh::Mesh;
use crate::engine::render::renderable::Renderable;

pub mod error;
pub mod mesh;

pub trait ComponentExt: Into<Component> {}

pub enum Component {
    Mesh(Mesh),
}

impl ComponentExt for Component {}

#[allow(unreachable_patterns)]
impl TryFrom<&Component> for Renderable {
    type Error = ();

    fn try_from(component: &Component) -> Result<Self, Self::Error> {
        match component {
            Component::Mesh(mesh) => Ok(mesh.clone().into()),
            _ => Err(()),
        }
    }
}

#[allow(irrefutable_let_patterns)]
impl<'a> TryFrom<&'a mut Component> for &'a mut Mesh {
    type Error = ();

    fn try_from(component: &'a mut Component) -> Result<Self, Self::Error> {
        if let Component::Mesh(mesh) = component {
            return Ok(mesh);
        }

        Err(())
    }
}
