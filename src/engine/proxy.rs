use crate::error::NeuclidioError;
use crate::event::NeuclidioEvent;
use crate::windowing::error::NeuclidioWindowingError;
use crate::windowing::event::{
    NeuclidioAddWindowResult, NeuclidioCloseWindowResult, NeuclidioWindowingEvent,
};
use bus::BusReader;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError};
use winit::event_loop::EventLoopProxy;
use winit::window::{WindowAttributes, WindowId};

pub struct NeuclidioEngineProxy {
    event_bus_reader: BusReader<NeuclidioEvent>,
    event_loop_proxy: EventLoopProxy<NeuclidioWindowingEvent>,
}

impl NeuclidioEngineProxy {
    pub(crate) fn new(
        event_bus_reader: BusReader<NeuclidioEvent>,
        event_loop_proxy: EventLoopProxy<NeuclidioWindowingEvent>,
    ) -> Self {
        Self {
            event_bus_reader,
            event_loop_proxy,
        }
    }

    pub fn poll_for_event(&mut self) -> Result<Option<NeuclidioEvent>, NeuclidioError> {
        match self.event_bus_reader.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(NeuclidioError::EventBusClosed),
        }
    }

    pub fn exit(&self) {
        let _ = self
            .event_loop_proxy
            .send_event(NeuclidioWindowingEvent::ExitEventLoop);
    }

    pub fn add_window(
        &self,
        window_attributes: WindowAttributes,
    ) -> Result<Receiver<NeuclidioAddWindowResult>, NeuclidioError> {
        let (sender, receiver) = mpsc::channel();
        self.event_loop_proxy
            .send_event(NeuclidioWindowingEvent::AddWindow(
                window_attributes,
                sender,
            ))
            .map_err(NeuclidioWindowingError::EventLoopClosed)?;

        Ok(receiver)
    }

    pub fn add_window_blocking(
        &self,
        window_attributes: WindowAttributes,
    ) -> NeuclidioAddWindowResult {
        self.add_window(window_attributes)?
            .recv()
            .map_err(|_| NeuclidioWindowingError::ChannelClosed)?
    }

    pub fn close_window(
        &self,
        window_id: WindowId,
    ) -> Result<Receiver<NeuclidioCloseWindowResult>, NeuclidioError> {
        let (sender, receiver) = mpsc::channel();
        self.event_loop_proxy
            .send_event(NeuclidioWindowingEvent::CloseWindow(window_id, sender))
            .map_err(NeuclidioWindowingError::EventLoopClosed)?;

        Ok(receiver)
    }

    pub fn close_window_blocking(&self, window_id: WindowId) -> NeuclidioCloseWindowResult {
        self.close_window(window_id)?
            .recv()
            .map_err(|_| NeuclidioWindowingError::ChannelClosed)?
    }
}
