use crate::error::NeuclidioResult;
use winit::window::{WindowAttributes, WindowId};

pub type CreateWindowResult = NeuclidioResult<WindowId>;
pub type CloseWindowResult = NeuclidioResult<()>;

#[derive(Debug)]
pub enum WindowingEvent {
    ExitEventLoop,
    CreateWindow(
        WindowAttributes,
        crossbeam_channel::Sender<CreateWindowResult>,
    ),
    CloseWindow(WindowId, crossbeam_channel::Sender<CloseWindowResult>),
}
