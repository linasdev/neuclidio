use crate::engine::EngineAppEvent;
use crate::engine::event::{EngineEvent, EngineInternalEvent};
use crate::engine::proxy::EngineProxy;
use crate::engine::proxy::request::EngineProxyRequest;
use crate::engine::render::windowing::event::WindowingEvent;
use crate::entity::Entity;
use crate::id::EntityId;
use bus::Bus;
use log::{info, warn};
use std::collections::HashMap;
use std::thread::{JoinHandle, spawn};
use winit::event_loop::EventLoopProxy;

pub struct EngineMainThread {
    pub event_bus: Bus<EngineEvent>,
    pub internal_event_bus: Bus<EngineInternalEvent>,
    pub event_loop_proxy: EventLoopProxy<WindowingEvent>,
    pub proxy_request_sender: crossbeam_channel::Sender<EngineProxyRequest>,
    pub proxy_request_receiver: crossbeam_channel::Receiver<EngineProxyRequest>,
    pub app_event_receiver: crossbeam_channel::Receiver<EngineAppEvent>,
    pub entities: HashMap<EntityId, Entity>,
}

impl EngineMainThread {
    pub fn spawn(mut self) -> JoinHandle<()> {
        spawn(move || {
            loop {
                crossbeam_channel::select! {
                    recv(self.proxy_request_receiver) -> proxy_request => {
                        if let Ok(proxy_request) = proxy_request {
                            self.handle_proxy_request(proxy_request);
                            continue;
                        }

                        break;
                    }
                    recv(self.app_event_receiver) -> app_event => {
                        if let Ok(app_event) = app_event {
                            self.handle_app_event(app_event);
                            continue;
                        }

                        break;
                    }
                }
            }
        })
    }

    fn handle_proxy_request(&mut self, proxy_request: EngineProxyRequest) {
        match proxy_request {
            EngineProxyRequest::AddProxy(proxy_sender) => {
                info!("Creating Neuclidio engine proxy");

                let proxy = EngineProxy::new(
                    self.event_bus.add_rx(),
                    self.event_loop_proxy.clone(),
                    self.proxy_request_sender.clone(),
                );

                if proxy_sender.send(proxy).is_err() {
                    log::error!("Failed to send engine proxy across proxy request channel");
                }
            }
            EngineProxyRequest::AddEntity(window_id, entity) => {
                let entity_id = entity.id();
                if entity.has_window_id(window_id) {
                    warn!(
                        "Entity with id '{entity_id}' is already added to window with id: {window_id:?}"
                    );
                    return;
                }

                entity.add_window_id(window_id);

                if self.entities.get(&entity_id).is_none() {
                    self.entities.insert(entity_id, entity.clone());
                }

                self.internal_event_bus
                    .broadcast(EngineInternalEvent::EntityAdded(window_id, entity));
            }
            EngineProxyRequest::RemoveEntity(entity) => {
                let entity_id = entity.id();
                if let Some(entity) = self.entities.remove(&entity_id) {
                    let window_ids = entity.drain_window_ids();
                    self.internal_event_bus
                        .broadcast(EngineInternalEvent::EntityRemoved(window_ids, entity));
                }
            }
            EngineProxyRequest::RemoveEntityById(entity_id) => {
                if let Some(entity) = self.entities.remove(&entity_id) {
                    let window_ids = entity.drain_window_ids();
                    self.internal_event_bus
                        .broadcast(EngineInternalEvent::EntityRemoved(window_ids, entity));
                }
            }
            EngineProxyRequest::HandleRenderableAdded(entity_id, renderable) => {
                if let Some(entity) = self.entities.get(&entity_id) {
                    self.internal_event_bus
                        .broadcast(EngineInternalEvent::RenderableAdded(
                            renderable,
                            entity.clone(),
                        ));
                }
            }
            EngineProxyRequest::HandleRenderableRemoved(entity_id, renderable) => {
                if let Some(entity) = self.entities.get(&entity_id) {
                    self.internal_event_bus
                        .broadcast(EngineInternalEvent::RenderableRemoved(
                            renderable,
                            entity.clone(),
                        ));
                }
            }
        }
    }

    fn handle_app_event(&mut self, app_event: EngineAppEvent) {
        match app_event {
            EngineAppEvent::WindowCreated(window) => self
                .internal_event_bus
                .broadcast(EngineInternalEvent::WindowCreated(window)),
            EngineAppEvent::WindowResized(window, new_window_size) => self
                .internal_event_bus
                .broadcast(EngineInternalEvent::WindowResized(window, new_window_size)),
            EngineAppEvent::WindowClosed(window) => {
                self.event_bus
                    .broadcast(EngineEvent::WindowClosed(window.id()));
                self.internal_event_bus
                    .broadcast(EngineInternalEvent::WindowClosed(window));
            }
        }
    }
}
