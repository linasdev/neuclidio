use crate::engine::render::windowing::event::NeuclidioWindowingEvent;
use winit::error::{EventLoopError, OsError};
use winit::event_loop::EventLoopClosed;

#[derive(Debug)]
pub enum NeuclidioWindowingError {
    ChannelClosed,
    WindowNotFound,
    EventLoopClosed(EventLoopClosed<NeuclidioWindowingEvent>),
    EventLoopError(EventLoopError),
    OsError(OsError),
}

impl From<EventLoopClosed<NeuclidioWindowingEvent>> for NeuclidioWindowingError {
    fn from(value: EventLoopClosed<NeuclidioWindowingEvent>) -> Self {
        NeuclidioWindowingError::EventLoopClosed(value)
    }
}

impl From<EventLoopError> for NeuclidioWindowingError {
    fn from(value: EventLoopError) -> Self {
        NeuclidioWindowingError::EventLoopError(value)
    }
}

impl From<OsError> for NeuclidioWindowingError {
    fn from(value: OsError) -> Self {
        NeuclidioWindowingError::OsError(value)
    }
}
