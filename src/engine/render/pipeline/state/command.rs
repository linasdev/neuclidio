use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use log::debug;
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, HasBuilder};

pub struct RenderPipelineCommandState {
    command_pool: vk::CommandPool,
    command_buffers: Option<Vec<vk::CommandBuffer>>,
}

impl RenderPipelineCommandState {
    pub fn new(neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<Self> {
        debug!(
            "Creating Vulkan command pool for window with id: {:?}",
            neuclidio_window.id
        );

        let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(neuclidio_window.queue_family_indices.graphics)
            .build();

        let command_pool = unsafe {
            neuclidio_window
                .logical_device
                .create_command_pool(&command_pool_create_info, None)?
        };

        Ok(Self {
            command_pool,
            command_buffers: None,
        })
    }

    pub fn prepare_for_reset(&mut self, neuclidio_window: &NeuclidioWindow) {
        let command_buffers = match self.command_buffers.take() {
            Some(command_buffers) => command_buffers,
            None => return,
        };

        debug!(
            "Freeing Vulkan command buffers for window with id: {:?}",
            neuclidio_window.id
        );

        unsafe {
            neuclidio_window
                .logical_device
                .free_command_buffers(self.command_pool, &command_buffers);
        }
    }

    pub fn reset(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()> {
        let logical_device = &neuclidio_window.logical_device;
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        debug!(
            "Creating Vulkan command buffers for window with id: {:?}",
            neuclidio_window.id
        );

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(swap_chain.image_count() as u32)
            .build();

        let command_buffers =
            unsafe { logical_device.allocate_command_buffers(&command_buffer_allocate_info)? };

        self.command_buffers = Some(command_buffers);

        Ok(())
    }

    pub fn record_command_buffer<CBR>(
        &mut self,
        neuclidio_window: &NeuclidioWindow,
        command_buffer_index: usize,
        mut command_buffer_recorder: CBR,
    ) -> NeuclidioResult<()>
    where
        CBR: FnMut(vk::CommandBuffer) -> NeuclidioResult<()>,
    {
        let logical_device = &neuclidio_window.logical_device;
        let command_buffer = self.command_buffer(command_buffer_index)?;

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

    pub fn destroy(self, neuclidio_window: &NeuclidioWindow) {
        debug!(
            "Destroying Vulkan command pool for window with id: {:?}",
            neuclidio_window.id
        );
        unsafe {
            neuclidio_window
                .logical_device
                .destroy_command_pool(self.command_pool, None);
        }
    }

    pub fn command_pool(&self) -> vk::CommandPool {
        self.command_pool
    }

    pub fn command_buffer(
        &self,
        command_buffer_index: usize,
    ) -> NeuclidioResult<vk::CommandBuffer> {
        self.command_buffers
            .as_ref()
            .map(|command_buffers| command_buffers[command_buffer_index])
            .ok_or(RenderPipelineError::Unprepared.into())
    }
}
