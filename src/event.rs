use winit::window::WindowId;

#[derive(Debug, Clone)]
pub enum NeuclidioEvent {
    WindowClosed(WindowId),
}
