use crate::component::renderable::Renderable;
use crate::engine::proxy::EngineProxy;
use crate::entity::Entity;
use crate::id::EntityId;
use winit::window::WindowId;

pub enum EngineProxyRequest {
    AddProxy(crossbeam_channel::Sender<EngineProxy>),
    AddEntity(WindowId, Entity),
    RemoveEntity(Entity),
    RemoveEntityById(EntityId),
    HandleRenderableAdded(EntityId, Renderable),
    HandleRenderableRemoved(EntityId, Renderable),
}
