use crate::engine::render::windowing::event::WindowingEvent;
use winit::error::{EventLoopError, OsError};
use winit::event_loop::EventLoopClosed;

#[derive(Debug)]
pub enum WindowingError {
    ChannelClosed,
    WindowNotFound,
    EventLoopClosed(EventLoopClosed<WindowingEvent>),
    EventLoopError(EventLoopError),
    OsError(OsError),
}

impl From<EventLoopClosed<WindowingEvent>> for WindowingError {
    fn from(value: EventLoopClosed<WindowingEvent>) -> Self {
        WindowingError::EventLoopClosed(value)
    }
}

impl From<EventLoopError> for WindowingError {
    fn from(value: EventLoopError) -> Self {
        WindowingError::EventLoopError(value)
    }
}

impl From<OsError> for WindowingError {
    fn from(value: OsError) -> Self {
        WindowingError::OsError(value)
    }
}
