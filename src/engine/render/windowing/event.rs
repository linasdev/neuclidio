use crate::error::NeuclidioResult;
use std::sync::mpsc::Sender;
use winit::window::{WindowAttributes, WindowId};

pub type AddWindowResult = NeuclidioResult<WindowId>;
pub type CloseWindowResult = NeuclidioResult<()>;

#[derive(Debug)]
pub enum WindowingEvent {
    ExitEventLoop,
    AddWindow(WindowAttributes, Sender<AddWindowResult>),
    CloseWindow(WindowId, Sender<CloseWindowResult>),
}
