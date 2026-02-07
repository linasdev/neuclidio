use winit::window::WindowId;

#[derive(Debug, Clone)]
pub enum Event {
    WindowClosed(WindowId),
}
