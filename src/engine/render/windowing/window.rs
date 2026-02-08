use crate::engine::render::windowing::queue_family_indices::QueueFamilyIndices;
use crate::engine::render::windowing::swap_chain::{SwapChain, SwapChainSupport};
use log::debug;
use vulkanalia::vk::{
    DeviceV1_0, ExtDebugUtilsExtensionInstanceCommands, InstanceV1_0,
    KhrSurfaceExtensionInstanceCommands,
};
use vulkanalia::{Device, Instance, vk};
use winit::window::WindowId;

pub struct NeuclidioWindow {
    pub id: WindowId,
    pub instance: Instance,
    pub logical_device: Device,
    pub queue_family_indices: QueueFamilyIndices,
    pub swap_chain_support: SwapChainSupport,
    pub physical_device: vk::PhysicalDevice,
    pub surface: vk::SurfaceKHR,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,

    #[cfg(debug_assertions)]
    pub debug_messenger: vk::DebugUtilsMessengerEXT,

    pub swap_chain: Option<SwapChain>,
}

impl Drop for NeuclidioWindow {
    fn drop(&mut self) {
        if let Some(swap_chain) = self.swap_chain.take() {
            swap_chain.destroy(self);
        }

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

        #[cfg(debug_assertions)]
        {
            debug!(
                "Destroying Vulkan debug utilities messenger for window with id: {:?}",
                self.id,
            );

            unsafe {
                self.instance
                    .destroy_debug_utils_messenger_ext(self.debug_messenger, None);
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
