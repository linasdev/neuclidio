use crate::engine::render::windowing::error::WindowingError;
use crate::engine::render::windowing::event::*;
use crate::error::{NeuclidioError, NeuclidioResult};
use crate::event::Event;
use bus::BusReader;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError};
use winit::event_loop::EventLoopProxy;
use winit::window::{WindowAttributes, WindowId};

pub struct EngineProxy {
    event_bus_reader: BusReader<Event>,
    event_loop_proxy: EventLoopProxy<WindowingEvent>,
}

impl EngineProxy {
    pub(crate) fn new(
        event_bus_reader: BusReader<Event>,
        event_loop_proxy: EventLoopProxy<WindowingEvent>,
    ) -> Self {
        Self {
            event_bus_reader,
            event_loop_proxy,
        }
    }

    pub fn poll_for_event(&mut self) -> NeuclidioResult<Option<Event>> {
        match self.event_bus_reader.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(NeuclidioError::EventBusClosed),
        }
    }

    pub fn exit(&self) {
        let _ = self
            .event_loop_proxy
            .send_event(WindowingEvent::ExitEventLoop);
    }

    pub fn add_window(
        &self,
        window_attributes: WindowAttributes,
    ) -> NeuclidioResult<Receiver<AddWindowResult>> {
        let (sender, receiver) = mpsc::channel();
        self.event_loop_proxy
            .send_event(WindowingEvent::AddWindow(window_attributes, sender))
            .map_err(WindowingError::from)?;

        Ok(receiver)
    }

    pub fn add_window_blocking(&self, window_attributes: WindowAttributes) -> AddWindowResult {
        self.add_window(window_attributes)?
            .recv()
            .map_err(|_| WindowingError::ChannelClosed)?
    }

    pub fn close_window(
        &self,
        window_id: WindowId,
    ) -> NeuclidioResult<Receiver<CloseWindowResult>> {
        let (sender, receiver) = mpsc::channel();
        self.event_loop_proxy
            .send_event(WindowingEvent::CloseWindow(window_id, sender))
            .map_err(WindowingError::from)?;

        Ok(receiver)
    }

    pub fn close_window_blocking(&self, window_id: WindowId) -> CloseWindowResult {
        self.close_window(window_id)?
            .recv()
            .map_err(|_| WindowingError::ChannelClosed)?
    }
}
