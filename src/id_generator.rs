use crate::component::mesh::MeshId;
use crate::engine::render::pipeline::common::state::allocator::RenderBufferId;
use crate::entity::EntityId;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ENTITY_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_RENDER_BUFFER_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_MESH_ID: AtomicU64 = AtomicU64::new(0);

pub struct IdGenerator {}

impl IdGenerator {
    pub fn generate_entity_id() -> EntityId {
        EntityId(NEXT_ENTITY_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn generate_render_buffer_id() -> RenderBufferId {
        RenderBufferId(NEXT_RENDER_BUFFER_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn generate_mesh_id() -> MeshId {
        MeshId(NEXT_MESH_ID.fetch_add(1, Ordering::Relaxed))
    }
}
