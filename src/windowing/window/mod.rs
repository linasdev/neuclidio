use crate::windowing::backend::WindowingBackendTrait;

#[cfg(feature = "display-protocol-x11")]
pub mod x11;

pub trait WindowTrait<B: WindowingBackendTrait> {}

#[derive(Debug)]
pub enum Window {
    #[cfg(feature = "display-protocol-x11")]
    X11(x11::X11Window),
}

impl<B: WindowingBackendTrait> WindowTrait<B> for Window {}
