use crate::engine::event::EngineEvent;
use crate::engine::proxy::error::EngineProxyError;
use crate::engine::proxy::request::EngineProxyRequest;
use crate::engine::render::windowing::error::WindowingError;
use crate::engine::render::windowing::event::*;
use crate::engine::thread::EngineThread;
use crate::entity::Entity;
use crate::error::{NeuclidioError, NeuclidioResult};
use crate::id::EntityId;
use bus::BusReader;
use log::info;
use std::sync::mpsc;
use winit::event_loop::EventLoopProxy;
use winit::window::{WindowAttributes, WindowId};

pub mod error;
pub mod request;

pub struct EngineProxy {
    event_bus_reader: BusReader<EngineEvent>,
    event_loop_proxy: EventLoopProxy<WindowingEvent>,
    proxy_request_sender: crossbeam_channel::Sender<EngineProxyRequest>,
}

impl EngineProxy {
    pub(crate) fn new(
        event_bus_reader: BusReader<EngineEvent>,
        event_loop_proxy: EventLoopProxy<WindowingEvent>,
        proxy_request_sender: crossbeam_channel::Sender<EngineProxyRequest>,
    ) -> Self {
        Self {
            event_bus_reader,
            event_loop_proxy,
            proxy_request_sender,
        }
    }

    pub fn add_proxy(&self) -> NeuclidioResult<EngineProxy> {
        let (sender, receiver) = crossbeam_channel::bounded(1);
        self.send_proxy_request(EngineProxyRequest::AddProxy(sender));

        receiver
            .recv()
            .map_err(|_| EngineProxyError::ChannelClosed.into())
    }

    pub fn add_thread<F, T>(&self, f: F) -> NeuclidioResult<EngineThread<T>>
    where
        F: FnOnce(EngineProxy) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        info!("Creating Neuclidio engine thread");
        let proxy = self.add_proxy()?;

        Ok(EngineThread::new(proxy, f))
    }

    pub fn add_entity(&self, window_id: WindowId, entity: Entity) {
        self.send_proxy_request(EngineProxyRequest::AddEntity(window_id, entity))
    }

    pub fn remove_entity(&self, entity: Entity) {
        self.send_proxy_request(EngineProxyRequest::RemoveEntity(entity))
    }

    pub fn remove_entity_by_id(&self, entity_id: EntityId) {
        self.send_proxy_request(EngineProxyRequest::RemoveEntityById(entity_id))
    }

    pub fn create_window(&self, window_attributes: WindowAttributes) -> CreateWindowResult {
        let (sender, receiver) = crossbeam_channel::bounded(1);
        self.event_loop_proxy
            .send_event(WindowingEvent::CreateWindow(window_attributes, sender))
            .map_err(WindowingError::from)?;

        receiver
            .recv()
            .map_err(|_| NeuclidioError::from(EngineProxyError::ChannelClosed))?
    }

    pub fn close_window(&self, window_id: WindowId) -> CloseWindowResult {
        let (sender, receiver) = crossbeam_channel::bounded(1);
        self.event_loop_proxy
            .send_event(WindowingEvent::CloseWindow(window_id, sender))
            .map_err(WindowingError::from)?;

        receiver
            .recv()
            .map_err(|_| NeuclidioError::from(EngineProxyError::ChannelClosed))?
    }

    pub fn poll_for_event(&mut self) -> NeuclidioResult<Option<EngineEvent>> {
        match self.event_bus_reader.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(EngineProxyError::EventBusClosed.into()),
        }
    }

    pub fn exit(&self) {
        let _ = self
            .event_loop_proxy
            .send_event(WindowingEvent::ExitEventLoop);
    }

    pub(crate) fn send_proxy_request(&self, event_proxy_request: EngineProxyRequest) {
        if self.proxy_request_sender.send(event_proxy_request).is_err() {
            panic!("Neuclidio engine proxy request channel closed");
        }
    }
}
