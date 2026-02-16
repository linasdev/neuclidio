use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::vulkan_context::VulkanContext;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use log::debug;
use std::collections::HashMap;
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, HasBuilder};
use winit::window::WindowId;

pub struct RenderPipelineCommandState {
    window_states: HashMap<WindowId, RenderPipelineCommandWindowState>,
    max_frames_in_flight: usize,
}

impl RenderPipelineCommandState {
    pub fn new(max_frames_in_flight: usize) -> NeuclidioResult<Self> {
        let window_states = HashMap::new();

        Ok(Self {
            window_states,
            max_frames_in_flight,
        })
    }

    pub fn reset_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<()> {
        if self.window_states.contains_key(&neuclidio_window.id) {
            return Ok(());
        }

        let window_state = RenderPipelineCommandWindowState::new(
            vulkan_context,
            neuclidio_window,
            self.max_frames_in_flight,
        )?;

        self.window_states.insert(neuclidio_window.id, window_state);

        Ok(())
    }

    pub fn clean_up_for_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) {
        if let Some(window_state) = self.window_states.remove(&neuclidio_window.id) {
            window_state.destroy(vulkan_context);
        }
    }

    pub fn destroy(self, vulkan_context: &VulkanContext) {
        for window_state in self.window_states.into_values() {
            window_state.destroy(vulkan_context);
        }
    }

    pub fn record_command_buffer<CBR>(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        frame_in_flight_index: usize,
        mut command_buffer_recorder: CBR,
    ) -> NeuclidioResult<()>
    where
        CBR: FnMut(vk::CommandBuffer) -> NeuclidioResult<()>,
    {
        let command_buffer = self.command_buffer(neuclidio_window, frame_in_flight_index)?;

        let logical_device = &vulkan_context.logical_device;

        unsafe {
            logical_device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;
        }

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();

        unsafe {
            logical_device.begin_command_buffer(command_buffer, &command_buffer_begin_info)?;
        }

        command_buffer_recorder(command_buffer)?;

        unsafe {
            logical_device.end_command_buffer(command_buffer)?;
        }

        Ok(())
    }

    pub fn command_buffer(
        &self,
        neuclidio_window: &NeuclidioWindow,
        frame_in_flight_index: usize,
    ) -> NeuclidioResult<vk::CommandBuffer> {
        self.window_states
            .get(&neuclidio_window.id)
            .map(|window_state| window_state.command_buffers[frame_in_flight_index])
            .ok_or(RenderPipelineError::Unprepared.into())
    }
}

struct RenderPipelineCommandWindowState {
    window_id: WindowId,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
}

impl RenderPipelineCommandWindowState {
    fn new(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        let window_id = neuclidio_window.id;

        debug!("Creating Vulkan graphics command pool for window with id: {window_id:?}");

        let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(vulkan_context.queue_family_indices.graphics)
            .build();

        let command_pool = unsafe {
            vulkan_context
                .logical_device
                .create_command_pool(&command_pool_create_info, None)?
        };

        debug!("Creating Vulkan graphics command buffers for window with id: {window_id:?}");

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(max_frames_in_flight as u32)
            .build();

        let command_buffers = unsafe {
            vulkan_context
                .logical_device
                .allocate_command_buffers(&command_buffer_allocate_info)?
        };

        Ok(Self {
            window_id,
            command_pool,
            command_buffers,
        })
    }

    fn destroy(self, vulkan_context: &VulkanContext) {
        debug!(
            "Destroying Vulkan graphics command pool for window with id: {:?}",
            self.window_id
        );

        unsafe {
            vulkan_context
                .logical_device
                .destroy_command_pool(self.command_pool, None);
        }
    }
}
