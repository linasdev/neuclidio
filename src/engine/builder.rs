use crate::engine::NeuclidioEngine;
use crate::engine::render::NeuclidioRenderEngine;
use crate::engine::render::builder::NeuclidioRenderEngineBuilder;
use crate::engine::render::windowing::error::NeuclidioWindowingError;
use crate::error::NeuclidioResult;
use bus::Bus;
use winit::event_loop::{ControlFlow, EventLoop};

const DEFAULT_EVENT_BUS_SIZE: usize = 64;

pub struct NeuclidioEngineBuilder {
    render_engine: Option<NeuclidioRenderEngine>,
    event_bus_size: usize,
}

impl Default for NeuclidioEngineBuilder {
    fn default() -> Self {
        Self {
            render_engine: None,
            event_bus_size: DEFAULT_EVENT_BUS_SIZE,
        }
    }
}

impl NeuclidioEngineBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_render_engine(mut self, render_engine: NeuclidioRenderEngine) -> Self {
        self.render_engine = Some(render_engine);
        self
    }

    pub fn with_event_bus_size(mut self, size: usize) -> Self {
        self.event_bus_size = size;
        self
    }

    pub fn build(self) -> NeuclidioResult<NeuclidioEngine> {
        let render_engine = match self.render_engine {
            Some(render_engine) => render_engine,
            None => NeuclidioRenderEngineBuilder::new().build()?,
        };

        let event_bus = Bus::new(self.event_bus_size);

        let event_loop = EventLoop::with_user_event()
            .build()
            .map_err(NeuclidioWindowingError::from)?;
        event_loop.set_control_flow(ControlFlow::Poll);

        Ok(NeuclidioEngine {
            render_engine,
            event_bus,
            event_loop,
        })
    }
}
