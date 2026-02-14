use crate::engine::proxy::EngineProxy;
use crate::engine::proxy::request::EngineProxyRequest;
use crate::engine::render::RenderEngine;
use crate::engine::render::windowing::error::WindowingError;
use crate::engine::thread::EngineThread;
use crate::entity::Entity;
use crate::error::NeuclidioResult;
use crate::event::Event;
use crate::id::EntityId;
use bus::Bus;
use log::{error, info, warn};
use render::windowing::event::WindowingEvent;
use std::collections::HashMap;
use std::sync::mpsc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

pub mod builder;
pub mod proxy;
pub mod render;
pub mod thread;

pub struct Engine {
    render_engine: RenderEngine,
    event_bus: Bus<Event>,
    event_loop: EventLoop<WindowingEvent>,
    proxy_request_sender: mpsc::Sender<EngineProxyRequest>,
    proxy_request_receiver: mpsc::Receiver<EngineProxyRequest>,
}

impl Engine {
    pub fn proxy(&mut self) -> EngineProxy {
        info!("Creating Neuclidio engine proxy");

        EngineProxy::new(
            self.event_bus.add_rx(),
            self.event_loop.create_proxy(),
            self.proxy_request_sender.clone(),
        )
    }

    pub fn thread<F, T>(&mut self, f: F) -> EngineThread<T>
    where
        F: FnOnce(EngineProxy) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        info!("Creating Neuclidio engine thread");

        EngineThread::new(self.proxy(), f)
    }

    pub fn run(self) -> NeuclidioResult<()> {
        info!("Running Neuclidio engine");

        let event_loop_proxy = self.event_loop.create_proxy();

        self.event_loop
            .run_app(&mut EngineApp {
                render_engine: self.render_engine,
                event_bus: self.event_bus,
                event_loop_proxy,
                proxy_request_sender: self.proxy_request_sender,
                proxy_request_receiver: self.proxy_request_receiver,
                windows: HashMap::new(),
                entities: HashMap::new(),
            })
            .map_err(WindowingError::from)?;

        Ok(())
    }
}

struct EngineApp {
    render_engine: RenderEngine,
    event_bus: Bus<Event>,
    event_loop_proxy: EventLoopProxy<WindowingEvent>,
    proxy_request_sender: mpsc::Sender<EngineProxyRequest>,
    proxy_request_receiver: mpsc::Receiver<EngineProxyRequest>,
    windows: HashMap<WindowId, Window>,
    entities: HashMap<EntityId, Entity>,
}

impl EngineApp {
    pub fn handle_proxy_request(&mut self, proxy_request: EngineProxyRequest) {
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

                if let Err(err) = self.render_engine.submit_entity(window_id, &entity) {
                    warn!(
                        "Failed to add entity with id '{entity_id}' to window with id '{window_id:?}' with error: {err:?}"
                    );
                }

                if self.entities.get(&entity_id).is_none() {
                    self.entities.insert(entity_id, entity);
                }
            }
            EngineProxyRequest::RemoveEntity(entity) => {
                let entity_id = entity.id();
                if let Some(entity) = self.entities.remove(&entity_id) {
                    entity.clear_window_ids();

                    if let Err(err) = self.render_engine.remove_entity(&entity) {
                        warn!("Failed to remove entity with id '{entity_id}' with error: {err:?}");
                    }
                }
            }
            EngineProxyRequest::RemoveEntityById(entity_id) => {
                if let Some(entity) = self.entities.remove(&entity_id) {
                    entity.clear_window_ids();

                    if let Err(err) = self.render_engine.remove_entity(&entity) {
                        warn!("Failed to remove entity with id '{entity_id}' with error: {err:?}");
                    }
                }
            }
            EngineProxyRequest::HandleRenderableAdded(entity_id, renderable) => {
                if let Some(entity) = self.entities.get(&entity_id) {
                    if let Err(err) = self
                        .render_engine
                        .handle_renderable_added(entity, renderable)
                    {
                        warn!(
                            "Failed to handle renderable added for entity with id '{entity_id}' with error: {err:?}"
                        );
                    }
                }
            }
            EngineProxyRequest::HandleRenderableRemoved(entity_id, renderable) => {
                if let Some(entity) = self.entities.get(&entity_id) {
                    if let Err(err) = self
                        .render_engine
                        .handle_renderable_removed(entity, renderable)
                    {
                        warn!(
                            "Failed to handle renderable removed for entity with id '{entity_id}' with error: {err:?}"
                        );
                    }
                }
            }
        }
    }
}

impl ApplicationHandler<WindowingEvent> for EngineApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: WindowingEvent) {
        match event {
            WindowingEvent::ExitEventLoop => event_loop.exit(),
            WindowingEvent::AddWindow(mut window_attributes, result_sender) => {
                if window_attributes.title.as_str() == "winit window" {
                    window_attributes = window_attributes.with_title("Neuclidio Example");
                }

                match event_loop.create_window(window_attributes) {
                    Ok(window) => {
                        let window_id = window.id();
                        info!("Added window with id: {:?}", window_id);

                        match self.render_engine.prepare_for_window(&window) {
                            Ok(_) => {
                                info!(
                                    "Prepared Vulkan instance for window with id: {:?}",
                                    window_id
                                );
                            }
                            Err(err) => {
                                error!("Failed to prepare Vulkan instance for window: {:?}", err);

                                if let Err(_) = result_sender.send(Err(err)) {
                                    error!("Failed to send result across windowing event channel");
                                }
                                return;
                            }
                        }

                        self.windows.insert(window_id, window);

                        let value = Ok(window_id);
                        if result_sender.send(value).is_err() {
                            log::error!("Failed to send result across windowing event channel");
                        }
                    }
                    Err(err) => {
                        error!("Failed to add window: {}", err);

                        let value = Err(WindowingError::OsError(err).into());
                        if let Err(_) = result_sender.send(value) {
                            error!("Failed to send result across windowing event channel");
                        }
                    }
                }
            }
            WindowingEvent::CloseWindow(window_id, result_sender) => {
                match self.windows.remove(&window_id) {
                    Some(window) => {
                        if let Err(err) = self.render_engine.clean_up_for_window(window_id) {
                            warn!(
                                "Failed to clean up for window with id '{window_id:?}' with error: {err:?}"
                            );
                            return;
                        }

                        drop(window);
                        info!("Closed window with id: {:?}", window_id);

                        if let Err(_) = result_sender.send(Ok(())) {
                            log::error!("Failed to send result across windowing event channel");
                        }
                    }
                    None => {
                        let err = WindowingError::WindowNotFound;
                        error!("Failed to close window: {:?}", err);

                        let value = Err(err.into());
                        if let Err(_) = result_sender.send(value) {
                            error!("Failed to send result across windowing event channel");
                        }
                    }
                }
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        loop {
            match self.proxy_request_receiver.try_recv() {
                Ok(request) => self.handle_proxy_request(request),
                Err(mpsc::TryRecvError::Disconnected) => {
                    panic!("Neuclidio engine proxy request channel closed");
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                if let Err(err) = self.render_engine.clean_up_for_window(window_id) {
                    warn!(
                        "Failed to clean up for window with id '{window_id:?}' with error: {err:?}"
                    );
                    return;
                }
                self.windows.remove(&window_id);
                info!("Closed window (due to user request) with id: {window_id:?}");

                self.event_bus.broadcast(Event::WindowClosed(window_id));
            }
            WindowEvent::Resized(_) => match self.windows.get(&window_id) {
                Some(window) => {
                    if let Err(err) = self.render_engine.handle_window_change(window) {
                        warn!(
                            "Failed to handle window change for window with id '{window_id:?}' with error: {err:?}"
                        );
                        return;
                    }
                }
                None => {
                    warn!("Can't find window by id '{window_id:?}' when processing window event")
                }
            },
            WindowEvent::RedrawRequested => match self.windows.get(&window_id) {
                Some(window) => {
                    if let Err(err) = self.render_engine.render_on_window(window) {
                        warn!(
                            "Failed to render on window with id '{window_id:?}' with error: {err:?}"
                        );
                        return;
                    }

                    window.request_redraw();
                }
                None => {
                    warn!("Can't find window by id '{window_id:?}' when processing window event")
                }
            },
            _ => {}
        }
    }
}
