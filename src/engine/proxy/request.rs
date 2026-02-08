use crate::entity::{Entity, EntityId};
use winit::window::WindowId;

pub enum EngineProxyRequest {
    AddEntity(WindowId, Entity),
    RemoveEntity(Entity),
    RemoveEntityById(EntityId),
}
