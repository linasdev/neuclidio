use crate::engine::render::vulkan_context::VulkanContext;
use crate::error::NeuclidioResult;
use log::debug;
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, HasBuilder};

pub struct RenderPipelineTransferState {
    command_pool: vk::CommandPool,
}

impl RenderPipelineTransferState {
    pub fn new(vulkan_context: &VulkanContext) -> NeuclidioResult<RenderPipelineTransferState> {
        debug!("Creating Vulkan transfer command pool");

        let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(vulkan_context.queue_family_indices.transfer)
            .build();

        let command_pool = unsafe {
            vulkan_context
                .logical_device
                .create_command_pool(&command_pool_create_info, None)?
        };

        Ok(Self { command_pool })
    }

    pub fn destroy(self, vulkan_context: &VulkanContext) {
        debug!("Destroying Vulkan transfer command pool");

        unsafe {
            vulkan_context
                .logical_device
                .destroy_command_pool(self.command_pool, None);
        }
    }

    pub fn command_pool(&self) -> vk::CommandPool {
        self.command_pool
    }
}
