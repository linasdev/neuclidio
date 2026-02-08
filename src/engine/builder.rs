use crate::engine::Engine;
use crate::engine::render::RenderEngine;
use crate::engine::render::builder::RenderEngineBuilder;
use crate::engine::render::windowing::error::WindowingError;
use crate::error::{NeuclidioError, NeuclidioResult};
use bus::Bus;
use std::sync::atomic::{AtomicBool, Ordering};
use winit::event_loop::{ControlFlow, EventLoop};

const DEFAULT_EVENT_BUS_SIZE: usize = 64;
static ENGINE_CREATED: AtomicBool = AtomicBool::new(false);

pub struct EngineBuilder {
    render_engine: Option<RenderEngine>,
    event_bus_size: usize,
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self {
            render_engine: None,
            event_bus_size: DEFAULT_EVENT_BUS_SIZE,
        }
    }
}

impl EngineBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_render_engine(mut self, render_engine: RenderEngine) -> Self {
        self.render_engine = Some(render_engine);
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

        let render_engine = match self.render_engine {
            Some(render_engine) => render_engine,
            None => RenderEngineBuilder::new().build()?,
        };

        let event_bus = Bus::new(self.event_bus_size);

        let event_loop = EventLoop::with_user_event()
            .build()
            .map_err(WindowingError::from)?;
        event_loop.set_control_flow(ControlFlow::Poll);

        Ok(Engine {
            render_engine,
            event_bus,
            event_loop,
        })
    }
}
