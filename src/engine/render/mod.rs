use crate::engine::render::windowing::window_manager::NeuclidioRenderEngineWindowManager;
use crate::error::NeuclidioResult;
use winit::dpi::PhysicalSize;
use winit::window::{Window, WindowId};

pub mod builder;
pub mod error;
pub mod pipeline;

pub(crate) mod windowing;

pub struct NeuclidioRenderEngine {
    window_manager: NeuclidioRenderEngineWindowManager,
}

impl NeuclidioRenderEngine {
    pub(crate) fn new(window_manager: NeuclidioRenderEngineWindowManager) -> Self {
        Self { window_manager }
    }

    pub(crate) fn prepare_for_window(&mut self, window: &Window) -> NeuclidioResult<()> {
        self.window_manager.prepare_for_window(window)
    }

    pub(crate) fn render_on_window(&mut self, window: &Window) -> NeuclidioResult<()> {
        self.window_manager.render_on_window(window)
    }

    pub(crate) fn handle_window_change(
        &mut self,
        window: &Window,
        new_window_size: PhysicalSize<u32>,
    ) -> NeuclidioResult<()> {
        self.window_manager
            .handle_window_change(window, Some(new_window_size))
    }

    pub(crate) fn clean_up_for_window(&mut self, window_id: WindowId) {
        self.window_manager.cleanup_for_window(window_id)
    }
}
