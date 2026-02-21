use crate::component::ComponentExt;
use crate::component::renderable::memory_allocation::RenderableMemoryAllocation;
use crate::component::renderable::mesh::Mesh;
use crate::id::{ComponentId, RenderBufferId};
use delegate::delegate;
use derive_more::From;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use vulkanalia::vk;
use winit::window::WindowId;

pub mod mesh;

pub(crate) mod memory_allocation;

pub(crate) trait RenderableExt: Into<Renderable> + Clone {
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

#[derive(From, Clone)]
pub enum Renderable {
    Mesh(Mesh),
}

impl RenderableExt for Renderable {
    delegate! {
        to match self {
            Renderable::Mesh(r) => r,
        } {
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
        }
    }
}

impl ComponentExt for Renderable {
    delegate! {
        to match self {
            Renderable::Mesh(r) => r,
        } {
            fn id(&self) -> ComponentId;
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
