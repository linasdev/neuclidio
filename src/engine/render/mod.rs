use crate::engine::render::windowing::window_manager::RenderEngineWindowManager;
use crate::error::NeuclidioResult;
use winit::window::{Window, WindowId};

pub mod builder;
pub mod error;
pub mod pipeline;

pub(crate) mod windowing;

pub struct RenderEngine {
    window_manager: RenderEngineWindowManager,
}

impl RenderEngine {
    pub(crate) fn new(window_manager: RenderEngineWindowManager) -> Self {
        Self { window_manager }
    }

    pub(crate) fn prepare_for_window(&mut self, window: &Window) -> NeuclidioResult<()> {
        self.window_manager.prepare_for_window(window)
    }

    pub(crate) fn render_on_window(&mut self, window: &Window) -> NeuclidioResult<()> {
        self.window_manager.render_on_window(window)
    }

    pub(crate) fn handle_window_change(&mut self, window: &Window) -> NeuclidioResult<()> {
        self.window_manager.handle_window_change(window)
    }

    pub(crate) fn clean_up_for_window(&mut self, window_id: WindowId) -> NeuclidioResult<()> {
        self.window_manager.cleanup_for_window(window_id)
    }
}
