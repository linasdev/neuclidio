use crate::engine::render::windowing::queue_family_indices::QueueFamilyIndices;
use log::debug;
use vulkanalia::vk::{DeviceV1_0, ExtDebugUtilsExtensionInstanceCommands, InstanceV1_0};
use vulkanalia::{Device, Entry, Instance, vk};
use vulkanalia_vma::Allocator;

pub struct VulkanContext {
    pub vulkan_entry: Entry,
    pub instance: Instance,
    pub logical_device: Device,
    pub allocator: Allocator,
    pub queue_family_indices: QueueFamilyIndices,
    pub physical_device: vk::PhysicalDevice,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub transfer_queue: vk::Queue,
    pub surface_format: vk::SurfaceFormatKHR,

    #[cfg(debug_assertions)]
    pub debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanContext {
    pub fn destroy(self) {
        debug!("Destroying Vulkan Memory Allocator");

        drop(self.allocator);

        debug!("Destroying Vulkan logical device");

        unsafe {
            self.logical_device.destroy_device(None);
        }

        #[cfg(debug_assertions)]
        {
            debug!("Destroying Vulkan debug utilities messenger");

            unsafe {
                self.instance
                    .destroy_debug_utils_messenger_ext(self.debug_messenger, None);
            }
        }

        debug!("Destroying Vulkan instance");

        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}
