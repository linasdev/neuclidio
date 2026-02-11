use crate::component::mesh::{Mesh, MeshId};
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};
use vulkanalia::vk;
use vulkanalia_vma::VirtualAllocation;

pub struct RenderableMemoryAllocation {
    pub render_buffer_index: usize,
    pub virtual_allocation: VirtualAllocation,
    pub offset: vk::DeviceSize,
    pub last_used_in_frame: Arc<Mutex<u64>>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub enum RenderableId {
    Mesh(MeshId),
}

pub trait RenderableExt: Into<Renderable> {
    fn id(&self) -> RenderableId;
    fn render_buffer_index(&self) -> Option<usize>;
    fn render_buffer_offset(&self) -> Option<vk::DeviceSize>;
    fn last_used_in_frame(&self) -> Option<Arc<Mutex<u64>>>;
    fn set_memory_allocation(
        &self,
        new_memory_allocation: Option<RenderableMemoryAllocation>,
    ) -> Option<RenderableMemoryAllocation>;
    fn load_into_staging_render_buffer(&self, staging_render_buffer_memory: *mut u8);
    fn index_count(&self) -> u64;
    fn index_offset(&self) -> vk::DeviceSize;
    fn size_in_render_buffer(&self) -> vk::DeviceSize {
        self.index_offset() + self.index_count() * size_of::<u32>() as u64
    }
}

#[derive(Clone)]
pub enum Renderable {
    Mesh(Mesh),
}

impl RenderableExt for Renderable {
    fn id(&self) -> RenderableId {
        match self {
            Renderable::Mesh(mesh) => mesh.id(),
        }
    }

    fn render_buffer_index(&self) -> Option<usize> {
        match self {
            Renderable::Mesh(mesh) => mesh.render_buffer_index(),
        }
    }

    fn render_buffer_offset(&self) -> Option<vk::DeviceSize> {
        match self {
            Renderable::Mesh(mesh) => mesh.render_buffer_offset(),
        }
    }

    fn last_used_in_frame(&self) -> Option<Arc<Mutex<u64>>> {
        match self {
            Renderable::Mesh(mesh) => mesh.last_used_in_frame(),
        }
    }

    fn set_memory_allocation(
        &self,
        new_memory_allocation: Option<RenderableMemoryAllocation>,
    ) -> Option<RenderableMemoryAllocation> {
        match self {
            Renderable::Mesh(mesh) => mesh.set_memory_allocation(new_memory_allocation),
        }
    }

    fn load_into_staging_render_buffer(&self, staging_render_buffer_memory: *mut u8) {
        match self {
            Renderable::Mesh(mesh) => {
                mesh.load_into_staging_render_buffer(staging_render_buffer_memory)
            }
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
