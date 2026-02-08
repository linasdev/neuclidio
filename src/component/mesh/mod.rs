use crate::component::{Component, ComponentExt};
use crate::engine::render::pipeline::common::vertex::Vertex;
use crate::engine::render::renderable::{Renderable, RenderableExt, RenderableId};
use std::slice;
use std::sync::Arc;

pub mod loader;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct MeshId(pub(crate) u64);

#[derive(Clone)]
pub struct Mesh {
    id: MeshId,
    vertices: Arc<Vec<Vertex>>,
    indices: Arc<Vec<u32>>,
}

impl RenderableExt for Mesh {
    fn id(&self) -> RenderableId {
        RenderableId::Mesh(self.id)
    }

    fn load_into_render_buffer(&self, mut render_buffer_memory: *mut u8) {
        let vertex_memory_slice = unsafe {
            let vertex_memory: *mut Vertex = render_buffer_memory.cast();
            slice::from_raw_parts_mut(vertex_memory, self.vertices.len())
        };

        vertex_memory_slice.copy_from_slice(&self.vertices);
        render_buffer_memory = unsafe { render_buffer_memory.add(self.index_offset() as usize) };

        let index_memory_slice = unsafe {
            let index_memory: *mut u32 = render_buffer_memory.cast();
            slice::from_raw_parts_mut(index_memory, self.indices.len())
        };

        index_memory_slice.copy_from_slice(&self.indices);
    }

    fn index_count(&self) -> u64 {
        self.indices.len() as u64
    }

    fn index_offset(&self) -> u64 {
        (self.vertices.len() * size_of::<Vertex>()) as u64
    }
}

impl ComponentExt for Mesh {}

impl From<Mesh> for Component {
    fn from(value: Mesh) -> Self {
        Component::Mesh(value)
    }
}

impl From<Mesh> for Renderable {
    fn from(value: Mesh) -> Self {
        Renderable::Mesh(value)
    }
}
