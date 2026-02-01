use crate::windowing::NeuclidioWindowingError;
use crate::windowing::backend::{WindowingBackend, WindowingBackendTrait};
use crate::windowing::config::WindowConfig;
use crate::windowing::window::Window;
use crate::windowing::window::x11::X11Window;
use breadx::display::DisplayConnection;
use breadx::prelude::*;
use breadx::protocol::xproto::{CreateWindowAux, Screen, Visualid, WindowClass};

pub struct X11WindowingBackend {
    connection: DisplayConnection,
}

impl X11WindowingBackend {
    pub fn new(display: &str) -> Result<Self, NeuclidioWindowingError> {
        let connection = DisplayConnection::connect(Some(display))?;

        Ok(Self { connection })
    }
}

impl WindowingBackendTrait for X11WindowingBackend {
    fn open(&mut self, config: &WindowConfig) -> Result<Window, NeuclidioWindowingError> {
        let window_id = self.connection.generate_xid()?;

        let screen = self.connection.default_screen();
        let visual_id = find_visual_for_depth(screen, config.bit_depth)
            .ok_or(NeuclidioWindowingError::UnsupportedBitDepth)?;

        self.connection.create_window_checked(
            config.bit_depth,
            window_id,
            screen.root,
            config.position_y,
            config.position_y,
            config.width,
            config.height,
            0,
            WindowClass::INPUT_OUTPUT,
            visual_id,
            CreateWindowAux::new().background_pixel(0),
        )?;

        self.connection.map_window_checked(window_id)?;

        Ok(X11Window::new(window_id).into())
    }

    fn close(&mut self, window: Window) -> Result<(), NeuclidioWindowingError> {
        #[allow(irrefutable_let_patterns)]
        if let Window::X11(window) = window {
            self.connection.destroy_window_checked(window.window_id)?;
            return Ok(());
        }

        Err(NeuclidioWindowingError::UnsupportedWindow(window))
    }

    fn poll_event(&mut self) -> Result<(), NeuclidioWindowingError> {
        match self.connection.poll_for_event()? {
            Some(event) => match event {
                _ => Ok(()),
            },
            None => Ok(()),
        }
    }
}

impl From<X11WindowingBackend> for WindowingBackend {
    fn from(value: X11WindowingBackend) -> Self {
        WindowingBackend::X11(value)
    }
}

impl From<breadx::Error> for NeuclidioWindowingError {
    fn from(value: breadx::Error) -> Self {
        Self::X11(value)
    }
}

fn find_visual_for_depth(screen: &Screen, target_depth: u8) -> Option<Visualid> {
    for depth_obj in &screen.allowed_depths {
        if depth_obj.depth == target_depth {
            return depth_obj.visuals.first().map(|v| v.visual_id);
        }
    }

    None
}
