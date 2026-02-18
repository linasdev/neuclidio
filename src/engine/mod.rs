use crate::engine::event::{EngineAppEvent, EngineInternalEvent};
use crate::engine::main_thread::EngineMainThread;
use crate::engine::proxy::EngineProxy;
use crate::engine::proxy::request::EngineProxyRequest;
use crate::engine::render::RenderEngine;
use crate::engine::render::windowing::error::WindowingError;
use crate::engine::thread::EngineThread;
use crate::error::NeuclidioResult;
use bus::Bus;
use event::EngineEvent;
use itertools::Itertools;
use log::{error, info};
use render::windowing::event::WindowingEvent;
use std::collections::HashMap;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

mod main_thread;

pub mod builder;
pub mod event;
pub mod proxy;
pub mod render;
pub mod thread;

pub struct Engine {
    render_engine: RenderEngine,
    event_bus: Bus<EngineEvent>,
    internal_event_bus: Bus<EngineInternalEvent>,
    event_loop: EventLoop<WindowingEvent>,
    proxy_request_sender: crossbeam_channel::Sender<EngineProxyRequest>,
    proxy_request_receiver: crossbeam_channel::Receiver<EngineProxyRequest>,
    app_event_sender: crossbeam_channel::Sender<EngineAppEvent>,
    app_event_receiver: crossbeam_channel::Receiver<EngineAppEvent>,
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
        let main_thread = EngineMainThread {
            event_bus: self.event_bus,
            internal_event_bus: self.internal_event_bus,
            event_loop_proxy,
            proxy_request_sender: self.proxy_request_sender,
            proxy_request_receiver: self.proxy_request_receiver,
            app_event_receiver: self.app_event_receiver,
            entities: HashMap::new(),
        };

        let main_thread_handle = main_thread.spawn();
        let render_thread_handle = self.render_engine.spawn();

        self.event_loop
            .run_app(&mut EngineApp {
                windows: HashMap::new(),
                app_event_sender: self.app_event_sender,
            })
            .map_err(WindowingError::from)?;

        render_thread_handle.join().unwrap();
        main_thread_handle.join().unwrap();

        Ok(())
    }
}

struct EngineApp {
    windows: HashMap<WindowId, Arc<Window>>,
    app_event_sender: crossbeam_channel::Sender<EngineAppEvent>,
}

impl EngineApp {
    fn handle_exit_event(&mut self, event_loop: &ActiveEventLoop) {
        let window_ids = self.windows.keys().copied().collect_vec();

        for window_id in window_ids.into_iter() {
            if let Err(err) = self.handle_window_closure(window_id) {
                error!("Failed to handle window closure: {:?}", err);
            }
        }

        event_loop.exit();
    }

    fn handle_window_creation(
        &mut self,
        event_loop: &ActiveEventLoop,
        mut window_attributes: WindowAttributes,
    ) -> NeuclidioResult<WindowId> {
        if window_attributes.title.as_str() == "winit window" {
            window_attributes = window_attributes.with_title("Neuclidio Example");
        }

        let window = event_loop
            .create_window(window_attributes)
            .map_err(|err| WindowingError::OsError(err))?;

        let window_id = window.id();
        let window = Arc::new(window);

        self.windows.insert(window_id, window.clone());
        self.send_engine_app_event(EngineAppEvent::WindowCreated(window));

        Ok(window_id)
    }

    fn handle_window_closure(&mut self, window_id: WindowId) -> NeuclidioResult<()> {
        let window = self
            .windows
            .remove(&window_id)
            .ok_or(WindowingError::WindowNotFound)?;

        self.send_engine_app_event(EngineAppEvent::WindowClosed(window));

        Ok(())
    }

    fn handle_window_resize(
        &mut self,
        window_id: WindowId,
        new_window_size: PhysicalSize<u32>,
    ) -> NeuclidioResult<()> {
        let window = self
            .windows
            .get(&window_id)
            .ok_or(WindowingError::WindowNotFound)?;
        self.send_engine_app_event(EngineAppEvent::WindowResized(
            window.clone(),
            new_window_size,
        ));
        Ok(())
    }

    fn send_engine_app_event(&self, value: EngineAppEvent) {
        if self.app_event_sender.send(value).is_err() {
            error!("Failed to send event across engine app event channel");
        }
    }
}

impl ApplicationHandler<WindowingEvent> for EngineApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: WindowingEvent) {
        match event {
            WindowingEvent::ExitEventLoop => self.handle_exit_event(event_loop),
            WindowingEvent::CreateWindow(window_attributes, result_sender) => {
                match self.handle_window_creation(event_loop, window_attributes) {
                    Ok(window_id) => {
                        info!("Created window with id: {window_id:?}");
                        if result_sender.send(Ok(window_id)).is_err() {
                            error!("Failed to send result across windowing event channel");
                        }
                    }
                    Err(err) => {
                        error!("Failed to handle window closure: {:?}", err);
                        if result_sender.send(Err(err)).is_err() {
                            error!("Failed to send result across windowing event channel");
                        }
                    }
                }
            }
            WindowingEvent::CloseWindow(window_id, result_sender) => {
                match self.handle_window_closure(window_id) {
                    Ok(_) => {
                        info!("Closing window (due to application request) with id: {window_id:?}");
                        if result_sender.send(Ok(())).is_err() {
                            error!("Failed to send result across windowing event channel");
                        }
                    }
                    Err(err) => {
                        error!("Failed to handle window closure: {:?}", err);
                        if result_sender.send(Err(err)).is_err() {
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
            WindowEvent::CloseRequested => match self.handle_window_closure(window_id) {
                Ok(_) => info!("Closing window (due to user request) with id: {window_id:?}"),
                Err(err) => error!("Failed to handle window closure: {:?}", err),
            },
            WindowEvent::Resized(new_window_size) => {
                if let Err(err) = self.handle_window_resize(window_id, new_window_size) {
                    error!("Failed to handle window closure: {:?}", err);
                }
            }
            WindowEvent::RedrawRequested => {} // We are handling rendering independently of the window manager
            _ => {}
        }
    }
}
