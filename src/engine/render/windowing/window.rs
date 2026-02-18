use crate::engine::render::vulkan_context::VulkanContext;
use crate::engine::render::windowing::swap_chain::SwapChain;
use log::debug;
use vulkanalia::vk;
use vulkanalia::vk::KhrSurfaceExtensionInstanceCommands;
use winit::dpi::PhysicalSize;
use winit::window::WindowId;

pub struct NeuclidioWindow {
    pub id: WindowId,
    pub surface: vk::SurfaceKHR,
    pub swap_chain: Option<SwapChain>,
    pub last_window_size: PhysicalSize<u32>,
}

impl NeuclidioWindow {
    pub fn destroy(mut self, vulkan_context: &VulkanContext) {
        if let Some(swap_chain) = self.swap_chain.take() {
            swap_chain.destroy(vulkan_context);
        }

        debug!(
            "Destroying Vulkan surface for window with id: {:?}",
            self.id
        );

        unsafe {
            vulkan_context
                .instance
                .destroy_surface_khr(self.surface, None);
        }
    }
}
