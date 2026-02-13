use winit::window::WindowId;

#[derive(Clone)]
pub enum Event {
    WindowClosed(WindowId),
}
