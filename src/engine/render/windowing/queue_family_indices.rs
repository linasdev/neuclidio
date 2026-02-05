use crate::engine::render::error::NeuclidioRenderError;
use crate::error::NeuclidioResult;
use std::collections::HashSet;
use vulkanalia::vk::{InstanceV1_0, KhrSurfaceExtensionInstanceCommands};
use vulkanalia::{Instance, vk};

#[derive(Copy, Clone, Debug)]
pub struct QueueFamilyIndices {
    pub graphics: u32,
    pub present: u32,
}

impl QueueFamilyIndices {
    pub fn new(
        instance: &Instance,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> NeuclidioResult<Self> {
        unsafe {
            let properties = instance.get_physical_device_queue_family_properties(physical_device);

            let graphics = properties
                .iter()
                .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                .map(|i| i as u32);

            let mut present = None;
            for (index, _) in properties.iter().enumerate() {
                if instance.get_physical_device_surface_support_khr(
                    physical_device,
                    index as u32,
                    surface,
                )? {
                    present = Some(index as u32);
                    break;
                }
            }

            if let (Some(graphics), Some(present)) = (graphics, present) {
                Ok(Self { graphics, present })
            } else {
                Err(NeuclidioRenderError::MissingRequiredQueueFamilies.into())
            }
        }
    }

    pub fn unique(&self) -> HashSet<u32> {
        let mut unique_indices = HashSet::new();
        unique_indices.insert(self.graphics);
        unique_indices.insert(self.present);
        unique_indices
    }
}
