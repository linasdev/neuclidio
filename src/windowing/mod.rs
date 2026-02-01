use crate::windowing::window::Window;

pub mod backend;
pub mod config;
pub mod window;

#[derive(Debug)]
pub enum NeuclidioWindowingError {
    NoProtocolAvailable,
    UnsupportedBitDepth,
    UnsupportedWindow(Window),

    #[cfg(feature = "display-protocol-x11")]
    X11(breadx::Error),
}
