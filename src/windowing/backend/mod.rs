use crate::windowing::NeuclidioWindowingError;
use crate::windowing::config::WindowConfig;
use crate::windowing::window::Window;

#[cfg(feature = "display-protocol-x11")]
pub mod x11;

pub trait WindowingBackendTrait {
    fn open(&mut self, config: &WindowConfig) -> Result<Window, NeuclidioWindowingError>;
    fn close(&mut self, window: Window) -> Result<(), NeuclidioWindowingError>;
    fn poll_event(&mut self) -> Result<(), NeuclidioWindowingError>;
}

pub enum WindowingBackend {
    #[cfg(feature = "display-protocol-x11")]
    X11(x11::X11WindowingBackend),
}

impl WindowingBackendTrait for WindowingBackend {
    fn open(&mut self, config: &WindowConfig) -> Result<Window, NeuclidioWindowingError> {
        match self {
            #[cfg(feature = "display-protocol-x11")]
            WindowingBackend::X11(backend) => backend.open(config),
        }
    }

    fn close(&mut self, window: Window) -> Result<(), NeuclidioWindowingError> {
        match self {
            #[cfg(feature = "display-protocol-x11")]
            WindowingBackend::X11(backend) => backend.close(window),
        }
    }

    fn poll_event(&mut self) -> Result<(), NeuclidioWindowingError> {
        match self {
            #[cfg(feature = "display-protocol-x11")]
            WindowingBackend::X11(backend) => backend.poll_event(),
        }
    }
}

pub fn get_available_windowing_backend() -> Result<WindowingBackend, NeuclidioWindowingError> {
    match std::env::consts::OS.to_lowercase().as_str() {
        #[cfg(feature = "platform-linux")]
        "linux" => get_available_windowing_backend_linux(),
        _ => Err(NeuclidioWindowingError::NoProtocolAvailable),
    }
}

#[cfg(feature = "platform-linux")]
fn get_available_windowing_backend_linux() -> Result<WindowingBackend, NeuclidioWindowingError> {
    // #[cfg(feature = "display-protocol-wayland")]
    // if let Ok(_) = std::env::var("WAYLAND_DISPLAY") {
    //     return Ok(wayland::WaylandWindowingBackend::new()?.into());
    // }

    #[cfg(feature = "display-protocol-x11")]
    if let Ok(display) = std::env::var("DISPLAY") {
        return Ok(x11::X11WindowingBackend::new(display.as_str())?.into());
    }

    Err(NeuclidioWindowingError::NoProtocolAvailable)
}
