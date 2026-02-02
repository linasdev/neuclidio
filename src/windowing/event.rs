use crate::error::NeuclidioError;
use std::sync::mpsc::Sender;
use winit::window::{WindowAttributes, WindowId};

pub type NeuclidioAddWindowResult = Result<WindowId, NeuclidioError>;
pub type NeuclidioCloseWindowResult = Result<(), NeuclidioError>;

#[derive(Debug)]
pub enum NeuclidioWindowingEvent {
    ExitEventLoop,
    AddWindow(WindowAttributes, Sender<NeuclidioAddWindowResult>),
    CloseWindow(WindowId, Sender<NeuclidioCloseWindowResult>),
}
