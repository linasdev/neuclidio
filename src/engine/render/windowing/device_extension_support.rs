use crate::engine::render::error::RenderError;
use crate::error::NeuclidioResult;
use std::collections::HashSet;
use vulkanalia::vk::InstanceV1_0;
use vulkanalia::{Instance, vk};

pub struct DeviceExtensionSupport {}

impl DeviceExtensionSupport {
    pub fn new(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        required_extensions: &[vk::ExtensionName],
    ) -> NeuclidioResult<()> {
        let extensions = unsafe {
            instance
                .enumerate_device_extension_properties(physical_device, None)?
                .iter()
                .map(|e| e.extension_name)
                .collect::<HashSet<_>>()
        };

        if required_extensions.iter().all(|e| extensions.contains(e)) {
            Ok(())
        } else {
            Err(RenderError::MissingDeviceExtensions.into())
        }
    }
}
