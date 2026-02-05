use crate::engine::render::error::NeuclidioRenderError;
use crate::error::NeuclidioResult;
use vulkanalia::vk::KhrSurfaceExtensionInstanceCommands;
use vulkanalia::{Instance, vk};

pub struct SwapChain {
    pub chain: vk::SwapchainKHR,
    pub extent: vk::Extent2D,
    pub image_format: vk::Format,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
}

#[derive(Clone, Debug)]
pub struct SwapChainSupport {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapChainSupport {
    pub fn new(
        instance: &Instance,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> NeuclidioResult<Self> {
        let capabilities = unsafe {
            instance.get_physical_device_surface_capabilities_khr(physical_device, surface)?
        };
        let formats =
            unsafe { instance.get_physical_device_surface_formats_khr(physical_device, surface)? };
        let present_modes = unsafe {
            instance.get_physical_device_surface_present_modes_khr(physical_device, surface)?
        };

        if formats.is_empty() || present_modes.is_empty() {
            return Err(NeuclidioRenderError::MissingSwapChainSupport.into());
        }

        Ok(Self {
            capabilities,
            formats,
            present_modes,
        })
    }
}
