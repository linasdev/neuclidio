use crate::component::renderable::Renderable;
use crate::entity::Entity;
use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::{Window, WindowId};

#[derive(Clone)]
pub enum EngineEvent {
    WindowClosed(WindowId),
}

#[derive(Clone)]
pub(crate) enum EngineInternalEvent {
    WindowCreated(Arc<Window>),
    WindowResized(Arc<Window>, PhysicalSize<u32>),
    WindowClosed(Arc<Window>),
    EntityAdded(WindowId, Entity),
    EntityRemoved(Vec<WindowId>, Entity),
    RenderableAdded(Renderable, Entity),
    RenderableRemoved(Renderable, Entity),
}

pub(crate) enum EngineAppEvent {
    WindowCreated(Arc<Window>),
    WindowResized(Arc<Window>, PhysicalSize<u32>),
    WindowClosed(Arc<Window>),
}
