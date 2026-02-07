use crate::component::mesh::{Mesh, MeshId};
use std::cmp::Ordering;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub enum RenderableId {
    Mesh(MeshId),
}

pub trait RenderableExt {
    fn id(&self) -> RenderableId;
    fn load_into_render_buffer(&self, render_buffer_memory: *mut u8);
    fn index_count(&self) -> u64;
    fn index_offset(&self) -> u64;
    fn size_in_render_buffer(&self) -> u64 {
        self.index_offset() + self.index_count() * size_of::<u32>() as u64
    }
}

pub enum Renderable {
    Mesh(Mesh),
}

impl RenderableExt for Renderable {
    fn id(&self) -> RenderableId {
        match self {
            Renderable::Mesh(mesh) => mesh.id(),
        }
    }

    fn load_into_render_buffer(&self, render_buffer_memory: *mut u8) {
        match self {
            Renderable::Mesh(mesh) => mesh.load_into_render_buffer(render_buffer_memory),
        }
    }

    fn index_count(&self) -> u64 {
        match self {
            Renderable::Mesh(mesh) => mesh.index_count(),
        }
    }

    fn index_offset(&self) -> u64 {
        match self {
            Renderable::Mesh(mesh) => mesh.index_offset(),
        }
    }
}

impl Ord for Renderable {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id().cmp(&other.id())
    }
}

impl Eq for Renderable {}

impl PartialOrd for Renderable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.id().partial_cmp(&other.id())
    }
}

impl PartialEq for Renderable {
    fn eq(&self, other: &Self) -> bool {
        self.id().eq(&other.id())
    }
}
