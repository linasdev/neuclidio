use crate::engine::Engine;
use crate::engine::render::RenderEngine;
use crate::engine::render::config::RenderEngineConfig;
use crate::engine::render::windowing::error::WindowingError;
use crate::error::{NeuclidioError, NeuclidioResult};
use bus::Bus;
use std::sync::atomic::{AtomicBool, Ordering};
use winit::event_loop::{ControlFlow, EventLoop};

const DEFAULT_EVENT_BUS_SIZE: usize = 64;
static ENGINE_CREATED: AtomicBool = AtomicBool::new(false);

pub struct EngineBuilder {
    render_engine_config: RenderEngineConfig,
    event_bus_size: usize,
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self {
            render_engine_config: RenderEngineConfig::default(),
            event_bus_size: DEFAULT_EVENT_BUS_SIZE,
        }
    }
}

impl EngineBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_render_engine_config(mut self, render_engine_config: RenderEngineConfig) -> Self {
        self.render_engine_config = render_engine_config;
        self
    }

    pub fn with_event_bus_size(mut self, size: usize) -> Self {
        self.event_bus_size = size;
        self
    }

    pub fn build(self) -> NeuclidioResult<Engine> {
        if ENGINE_CREATED.fetch_or(true, Ordering::Relaxed) {
            return Err(NeuclidioError::EngineAlreadyExists);
        }

        let event_bus = Bus::new(self.event_bus_size);
        let mut internal_event_bus = Bus::new(self.event_bus_size);

        let application_info = self.render_engine_config.application_info();
        let render_engine = RenderEngine::new(internal_event_bus.add_rx(), application_info);

        let event_loop = EventLoop::with_user_event()
            .build()
            .map_err(WindowingError::from)?;
        event_loop.set_control_flow(ControlFlow::Poll);

        let (proxy_request_sender, proxy_request_receiver) = crossbeam_channel::unbounded();
        let (app_event_sender, app_event_receiver) = crossbeam_channel::unbounded();

        Ok(Engine {
            render_engine,
            event_bus,
            internal_event_bus,
            event_loop,
            proxy_request_sender,
            proxy_request_receiver,
            app_event_sender,
            app_event_receiver,
        })
    }
}
