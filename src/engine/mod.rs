use crate::engine::proxy::NeuclidioEngineProxy;
use crate::engine::render::NeuclidioRenderEngine;
use crate::engine::render::windowing::error::NeuclidioWindowingError;
use crate::engine::thread::NeuclidioEngineThread;
use crate::error::NeuclidioResult;
use crate::event::NeuclidioEvent;
use bus::Bus;
use log::{error, info, warn};
use render::windowing::event::NeuclidioWindowingEvent;
use std::collections::HashMap;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

pub mod builder;
pub mod proxy;
pub mod render;
pub mod thread;

pub struct NeuclidioEngine {
    render_engine: NeuclidioRenderEngine,
    event_bus: Bus<NeuclidioEvent>,
    event_loop: EventLoop<NeuclidioWindowingEvent>,
}

impl NeuclidioEngine {
    pub fn proxy(&mut self) -> NeuclidioEngineProxy {
        NeuclidioEngineProxy::new(self.event_bus.add_rx(), self.event_loop.create_proxy())
    }

    pub fn thread<F, T>(&mut self, f: F) -> NeuclidioEngineThread<T>
    where
        F: FnOnce(NeuclidioEngineProxy) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        NeuclidioEngineThread::new(self.proxy(), f)
    }

    pub fn run(self) -> NeuclidioResult<()> {
        self.event_loop
            .run_app(&mut NeuclidioEngineApp {
                render_engine: self.render_engine,
                event_bus: self.event_bus,
                windows: HashMap::new(),
            })
            .map_err(NeuclidioWindowingError::EventLoopError)?;

        Ok(())
    }
}

struct NeuclidioEngineApp {
    render_engine: NeuclidioRenderEngine,
    event_bus: Bus<NeuclidioEvent>,
    windows: HashMap<WindowId, Window>,
}

impl ApplicationHandler<NeuclidioWindowingEvent> for NeuclidioEngineApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: NeuclidioWindowingEvent) {
        match event {
            NeuclidioWindowingEvent::ExitEventLoop => event_loop.exit(),
            NeuclidioWindowingEvent::AddWindow(mut window_attributes, result_sender) => {
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
                        if let Err(_) = result_sender.send(value) {
                            log::error!("Failed to send result across windowing event channel");
                        }
                    }
                    Err(err) => {
                        error!("Failed to add window: {}", err);

                        let value = Err(NeuclidioWindowingError::OsError(err).into());
                        if let Err(_) = result_sender.send(value) {
                            error!("Failed to send result across windowing event channel");
                        }
                    }
                }
            }
            NeuclidioWindowingEvent::CloseWindow(window_id, result_sender) => {
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
                        let err = NeuclidioWindowingError::WindowNotFound;
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

                self.event_bus
                    .broadcast(NeuclidioEvent::WindowClosed(window_id));
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
