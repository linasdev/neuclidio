use crate::component::{Component, ComponentExt};
use crate::engine::render::pipeline::common::vertex::Vertex;
use crate::engine::render::renderable::{
    Renderable, RenderableExt, RenderableId, RenderableMemoryAllocation,
};
use std::slice;
use std::sync::{Arc, Mutex};
use vulkanalia::vk;

pub mod loader;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct MeshId(pub(crate) u64);

#[derive(Clone)]
pub struct Mesh {
    id: MeshId,
    vertices: Arc<Vec<Vertex>>,
    indices: Arc<Vec<u32>>,
    memory_allocation: Arc<Mutex<Option<RenderableMemoryAllocation>>>,
}

impl RenderableExt for Mesh {
    fn id(&self) -> RenderableId {
        RenderableId::Mesh(self.id)
    }

    fn render_buffer_index(&self) -> Option<usize> {
        self.memory_allocation
            .lock()
            .unwrap()
            .as_ref()
            .map(|memory_allocation| memory_allocation.render_buffer_index)
    }

    fn render_buffer_offset(&self) -> Option<vk::DeviceSize> {
        self.memory_allocation
            .lock()
            .unwrap()
            .as_ref()
            .map(|memory_allocation| memory_allocation.offset)
    }

    fn last_used_in_frame(&self) -> Option<Arc<Mutex<u64>>> {
        self.memory_allocation
            .lock()
            .unwrap()
            .as_ref()
            .map(|memory_allocation| memory_allocation.last_used_in_frame.clone())
    }

    fn set_memory_allocation(
        &self,
        new_memory_allocation: Option<RenderableMemoryAllocation>,
    ) -> Option<RenderableMemoryAllocation> {
        let mut memory_allocation = self.memory_allocation.lock().unwrap();
        let old_memory_allocation = memory_allocation.take();

        if let Some(new_memory_allocation) = new_memory_allocation {
            memory_allocation.replace(new_memory_allocation);
        }

        old_memory_allocation
    }

    fn load_into_staging_render_buffer(&self, mut staging_render_buffer_memory: *mut u8) {
        let vertex_memory_slice = unsafe {
            let vertex_memory: *mut Vertex = staging_render_buffer_memory.cast();
            slice::from_raw_parts_mut(vertex_memory, self.vertices.len())
        };

        vertex_memory_slice.copy_from_slice(&self.vertices);
        staging_render_buffer_memory =
            unsafe { staging_render_buffer_memory.add(self.index_offset() as usize) };

        let index_memory_slice = unsafe {
            let index_memory: *mut u32 = staging_render_buffer_memory.cast();
            slice::from_raw_parts_mut(index_memory, self.indices.len())
        };

        index_memory_slice.copy_from_slice(&self.indices);
    }

    fn index_count(&self) -> u64 {
        self.indices.len() as u64
    }

    fn index_offset(&self) -> vk::DeviceSize {
        (self.vertices.len() * size_of::<Vertex>()) as u64
    }

    fn size_in_render_buffer(&self) -> vk::DeviceSize {
        self.index_offset() + self.index_count() * size_of::<u32>() as u64
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
