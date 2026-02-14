use crate::component::mesh::Mesh;
use crate::id::{RenderBufferId, RenderableId};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use vulkanalia::vk;
use vulkanalia_vma::VirtualAllocation;
use winit::window::WindowId;

pub struct RenderableMemoryAllocation {
    pub render_buffer_id: RenderBufferId,
    pub virtual_allocation: VirtualAllocation,
    pub offset: vk::DeviceSize,
    pub last_used_in_frame: Arc<Mutex<HashMap<WindowId, u64>>>,
}

pub trait RenderableExt: Into<Renderable> {
    fn id(&self) -> RenderableId;
    fn render_buffer_id(&self) -> Option<RenderBufferId>;
    fn render_buffer_offset(&self) -> Option<vk::DeviceSize>;
    fn last_used_in_frame(&self) -> Option<Arc<Mutex<HashMap<WindowId, u64>>>>;
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

    fn render_buffer_id(&self) -> Option<RenderBufferId> {
        match self {
            Renderable::Mesh(mesh) => mesh.render_buffer_id(),
        }
    }

    fn render_buffer_offset(&self) -> Option<vk::DeviceSize> {
        match self {
            Renderable::Mesh(mesh) => mesh.render_buffer_offset(),
        }
    }

    fn last_used_in_frame(&self) -> Option<Arc<Mutex<HashMap<WindowId, u64>>>> {
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

impl Eq for Renderable {}

impl PartialEq for Renderable {
    fn eq(&self, other: &Self) -> bool {
        self.id().eq(&other.id())
    }
}

impl Hash for Renderable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}
