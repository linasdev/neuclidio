use crate::engine::render::error::RenderError;
use crate::error::NeuclidioResult;
use vulkanalia::vk::{HasBuilder, InstanceV1_1};
use vulkanalia::{Instance, vk};

pub struct SynchronizationSupport {}

impl SynchronizationSupport {
    pub fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> NeuclidioResult<Self> {
        let mut timeline_semaphore_features =
            vk::PhysicalDeviceTimelineSemaphoreFeatures::default();
        let mut synchronization2_features = vk::PhysicalDeviceSynchronization2Features::default();
        let mut physical_device_properties2 = vk::PhysicalDeviceFeatures2::builder()
            .push_next(&mut timeline_semaphore_features)
            .push_next(&mut synchronization2_features)
            .build();

        unsafe {
            instance
                .get_physical_device_features2(physical_device, &mut physical_device_properties2);
        }

        if timeline_semaphore_features.timeline_semaphore == vk::FALSE {
            return Err(RenderError::MissingTimelineSemaphoreSupport.into());
        }

        if synchronization2_features.synchronization2 == vk::FALSE {
            return Err(RenderError::MissingSynchronization2Support.into());
        }

        Ok(Self {})
    }
}
