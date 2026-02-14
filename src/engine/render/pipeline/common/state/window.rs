use crate::engine::render::pipeline::common::state::command::RenderPipelineCommandState;
use crate::engine::render::pipeline::common::state::synchronization::RenderPipelineSynchronizationState;
use crate::engine::render::vulkan_context::VulkanContext;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;

pub struct RenderPipelineWindowState {
    pub synchronization_state: RenderPipelineSynchronizationState,
    pub command_state: RenderPipelineCommandState,
}

impl RenderPipelineWindowState {
    pub fn new(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        let synchronization_state = RenderPipelineSynchronizationState::new(
            vulkan_context,
            neuclidio_window,
            max_frames_in_flight,
        )?;
        let command_state = RenderPipelineCommandState::new(vulkan_context, neuclidio_window)?;

        Ok(Self {
            synchronization_state,
            command_state,
        })
    }
}
