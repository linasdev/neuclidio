use crate::engine::proxy::EngineProxy;
use crate::engine::render::renderable::Renderable;
use crate::entity::{Entity, EntityId};
use std::sync::mpsc;
use winit::window::WindowId;

pub enum EngineProxyRequest {
    AddProxy(mpsc::Sender<EngineProxy>),
    AddEntity(WindowId, Entity),
    RemoveEntity(Entity),
    RemoveEntityById(EntityId),
    HandleRenderableAdded(EntityId, Renderable),
    HandleRenderableRemoved(EntityId, Renderable),
}
