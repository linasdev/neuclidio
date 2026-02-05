use crate::engine::render::pipeline::NeuclidioRenderPipeline;
use crate::engine::render::windowing::queue_family_indices::QueueFamilyIndices;
use crate::engine::render::windowing::swap_chain::{SwapChain, SwapChainSupport};
use crate::error::NeuclidioResult;
use log::{debug, warn};
use vulkanalia::vk::{
    DeviceV1_0, ExtDebugUtilsExtensionInstanceCommands, InstanceV1_0,
    KhrSurfaceExtensionInstanceCommands, KhrSwapchainExtensionDeviceCommands,
};
use vulkanalia::{Device, Instance, vk};
use winit::window::WindowId;

pub struct NeuclidioWindow {
    pub id: WindowId,
    pub instance: Instance,
    pub logical_device: Device,
    pub queue_family_indices: QueueFamilyIndices,
    pub swap_chain_support: SwapChainSupport,
    pub debug_messenger: Option<vk::DebugUtilsMessengerEXT>,

    pub surface: vk::SurfaceKHR,
    pub swap_chain: SwapChain,
    pub pipeline: Box<dyn NeuclidioRenderPipeline>,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
}

impl NeuclidioWindow {
    pub fn render(&mut self) -> NeuclidioResult<()> {
        self.pipeline.render(
            &self.logical_device,
            &self.swap_chain,
            self.graphics_queue,
            self.present_queue,
        )?;

        Ok(())
    }

    pub fn destroy_swap_chain(&self, skip_freeing_command_buffers: bool) {
        self.pipeline
            .destroy_frame_buffers(&self.logical_device, skip_freeing_command_buffers);

        debug!(
            "Destroying Vulkan swap chain image views for window with id: {:?}",
            self.id,
        );
        for swap_chain_image_view in self.swap_chain.image_views.iter() {
            unsafe {
                self.logical_device
                    .destroy_image_view(*swap_chain_image_view, None);
            }
        }

        debug!(
            "Destroying Vulkan swap chain for window with id: {:?}",
            self.id,
        );
        unsafe {
            self.logical_device
                .destroy_swapchain_khr(self.swap_chain.chain, None);
        }
    }
}

impl Drop for NeuclidioWindow {
    fn drop(&mut self) {
        let device_wait_result = unsafe { self.logical_device.device_wait_idle() };

        if let Err(err) = device_wait_result {
            warn!("Failed to wait for device to be idle: {err}");
        }

        self.pipeline.destroy(&self.logical_device);
        self.destroy_swap_chain(true);

        debug!(
            "Destroying Vulkan logical device for window with id: {:?}",
            self.id
        );
        unsafe {
            self.logical_device.destroy_device(None);
        }

        debug!(
            "Destroying Vulkan surface for window with id: {:?}",
            self.id
        );
        unsafe {
            self.instance.destroy_surface_khr(self.surface, None);
        }

        if let Some(debug_messenger) = self.debug_messenger {
            debug!(
                "Destroying Vulkan debug utilities messenger for window with id: {:?}",
                self.id,
            );

            unsafe {
                self.instance
                    .destroy_debug_utils_messenger_ext(debug_messenger, None);
            }
        }

        debug!(
            "Destroying Vulkan instance for window with id: {:?}",
            self.id
        );
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}
