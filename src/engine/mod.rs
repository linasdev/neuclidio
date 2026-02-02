use crate::engine::proxy::NeuclidioEngineProxy;
use crate::engine::thread::NeuclidioEngineThread;
use crate::error::NeuclidioError;
use crate::event::NeuclidioEvent;
use crate::windowing::error::NeuclidioWindowingError;
use crate::windowing::event::NeuclidioWindowingEvent;
use bus::Bus;
use log::{error, info, warn};
use std::collections::HashMap;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

pub mod proxy;
pub mod thread;

const DEFAULT_EVENT_BUS_SIZE: usize = 64;

pub struct NeuclidioEngine {
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

    pub fn run(self) -> Result<(), NeuclidioError> {
        self.event_loop
            .run_app(&mut NeuclidioEngineApp {
                event_bus: self.event_bus,
                windows: HashMap::new(),
            })
            .map_err(NeuclidioWindowingError::EventLoopError)?;

        Ok(())
    }
}

struct NeuclidioEngineApp {
    event_bus: Bus<NeuclidioEvent>,
    windows: HashMap<WindowId, Window>,
}

impl ApplicationHandler<NeuclidioWindowingEvent> for NeuclidioEngineApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: NeuclidioWindowingEvent) {
        match event {
            NeuclidioWindowingEvent::ExitEventLoop => event_loop.exit(),
            NeuclidioWindowingEvent::AddWindow(window_attributes, result_sender) => {
                match event_loop.create_window(window_attributes) {
                    Ok(window) => {
                        let window_id = window.id();
                        self.windows.insert(window_id, window);
                        info!("Added window with id: {:?}", window_id);

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
                self.windows.remove(&window_id);
                info!(
                    "Closed window (due to user request) with id: {:?}",
                    window_id
                );

                self.event_bus
                    .broadcast(NeuclidioEvent::WindowClosed(window_id));
            }
            WindowEvent::RedrawRequested => match self.windows.get(&window_id) {
                Some(window) => {
                    window.request_redraw();
                }
                None => warn!(
                    "Can't find window by id '{:?}' when processing window event",
                    window_id
                ),
            },
            _ => {}
        }
    }
}

#[derive(Default)]
pub struct NeuclidioEngineBuilder {
    event_bus_size: Option<usize>,
}

impl NeuclidioEngineBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> Result<NeuclidioEngine, NeuclidioError> {
        let event_bus = Bus::new(self.event_bus_size.unwrap_or(DEFAULT_EVENT_BUS_SIZE));

        let event_loop = EventLoop::with_user_event()
            .build()
            .map_err(NeuclidioWindowingError::from)?;
        event_loop.set_control_flow(ControlFlow::Poll);

        Ok(NeuclidioEngine {
            event_bus,
            event_loop,
        })
    }
}
