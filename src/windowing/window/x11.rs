use crate::windowing::backend::x11::X11WindowingBackend;
use crate::windowing::window::{Window, WindowTrait};

#[derive(Debug)]
pub struct X11Window {
    pub(crate) window_id: u32,
}

impl X11Window {
    pub(crate) fn new(window_id: u32) -> Self {
        X11Window { window_id }
    }
}

impl WindowTrait<X11WindowingBackend> for X11Window {}

impl From<X11Window> for Window {
    fn from(value: X11Window) -> Self {
        Window::X11(value)
    }
}
