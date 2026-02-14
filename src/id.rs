use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ENTITY_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_RENDER_BUFFER_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_MESH_ID: AtomicU64 = AtomicU64::new(0);

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct EntityId(u64);

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct RenderBufferId(u64);

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct MeshId(u64);

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum RenderableId {
    Mesh(MeshId),
}

impl EntityId {
    pub fn new() -> Self {
        Self(NEXT_ENTITY_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl RenderBufferId {
    pub fn new() -> Self {
        Self(NEXT_RENDER_BUFFER_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl MeshId {
    pub fn new() -> Self {
        Self(NEXT_MESH_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Display for EntityId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for RenderBufferId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for MeshId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
