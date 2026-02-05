use crate::error::NeuclidioResult;
use std::sync::mpsc::Sender;
use winit::window::{WindowAttributes, WindowId};

pub type NeuclidioAddWindowResult = NeuclidioResult<WindowId>;
pub type NeuclidioCloseWindowResult = NeuclidioResult<()>;

#[derive(Debug)]
pub enum NeuclidioWindowingEvent {
    ExitEventLoop,
    AddWindow(WindowAttributes, Sender<NeuclidioAddWindowResult>),
    CloseWindow(WindowId, Sender<NeuclidioCloseWindowResult>),
}
