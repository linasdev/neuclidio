use crate::engine::render::error::RenderError;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use itertools::Itertools;
use log::debug;
use vulkanalia::vk::{
    DeviceV1_0, Handle, HasBuilder, KhrSurfaceExtensionInstanceCommands,
    KhrSwapchainExtensionDeviceCommands,
};
use vulkanalia::{Instance, vk};
use winit::window::{Window, WindowId};

pub struct SwapChain {
    window_id: WindowId,
    chain: vk::SwapchainKHR,
    extent: vk::Extent2D,
    image_format: vk::Format,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
}

impl SwapChain {
    pub fn new(
        window: &Window,
        neuclidio_window: &NeuclidioWindow,
        preferred_image_count: u32,
        preferred_present_modes: &[vk::PresentModeKHR],
    ) -> NeuclidioResult<Self> {
        let logical_device = &neuclidio_window.logical_device;
        let queue_family_indices = neuclidio_window.queue_family_indices;
        let swap_chain_support = SwapChainSupport::new(
            &neuclidio_window.instance,
            neuclidio_window.surface,
            neuclidio_window.physical_device,
        )?;
        let surface_capabilities = swap_chain_support.capabilities;
        let image_count = Self::get_image_count(surface_capabilities, preferred_image_count);
        let surface_format = Self::get_surface_format(&swap_chain_support.formats)?;
        let image_format = surface_format.format;
        let image_color_space = surface_format.color_space;
        let extent = Self::get_extent(window, surface_capabilities);

        let (image_sharing_mode, queue_family_indices) = if queue_family_indices.unique().len() > 1
        {
            (
                vk::SharingMode::CONCURRENT,
                vec![queue_family_indices.graphics, queue_family_indices.present],
            )
        } else {
            (vk::SharingMode::EXCLUSIVE, vec![])
        };

        let present_mode =
            Self::get_present_mode(&swap_chain_support.present_modes, preferred_present_modes)?;

        let swap_chain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(neuclidio_window.surface)
            .min_image_count(image_count)
            .image_format(image_format)
            .image_color_space(image_color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(image_sharing_mode)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null())
            .build();

        let (chain, images) = unsafe {
            let chain = logical_device.create_swapchain_khr(&swap_chain_create_info, None)?;
            let images = logical_device.get_swapchain_images_khr(chain)?;

            (chain, images)
        };
        let image_views = Self::create_image_views(neuclidio_window, &images, image_format)?;

        let swap_chain = SwapChain {
            window_id: window.id(),
            chain,
            extent,
            image_format,
            images,
            image_views,
        };

        Ok(swap_chain)
    }

    pub fn destroy(self, neuclidio_window: &NeuclidioWindow) {
        debug!(
            "Destroying Vulkan swap chain image views for window with id: {:?}",
            self.window_id,
        );
        for swap_chain_image_view in self.image_views.iter() {
            unsafe {
                neuclidio_window
                    .logical_device
                    .destroy_image_view(*swap_chain_image_view, None);
            }
        }

        debug!(
            "Destroying Vulkan swap chain for window with id: {:?}",
            self.window_id,
        );
        unsafe {
            neuclidio_window
                .logical_device
                .destroy_swapchain_khr(self.chain, None);
        }
    }

    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    pub fn chain(&self) -> vk::SwapchainKHR {
        self.chain
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub fn image_format(&self) -> vk::Format {
        self.image_format
    }

    pub fn image_views(&self) -> &[vk::ImageView] {
        &self.image_views
    }

    fn get_image_count(
        surface_capabilities: vk::SurfaceCapabilitiesKHR,
        preferred_image_count: u32,
    ) -> u32 {
        if surface_capabilities.max_image_count != 0 {
            preferred_image_count.clamp(
                surface_capabilities.min_image_count,
                surface_capabilities.max_image_count,
            )
        } else {
            preferred_image_count.max(surface_capabilities.min_image_count)
        }
    }

    fn get_surface_format(
        available_surface_formats: &[vk::SurfaceFormatKHR],
    ) -> NeuclidioResult<vk::SurfaceFormatKHR> {
        available_surface_formats
            .iter()
            .cloned()
            .find_or_first(|surface_format| {
                surface_format.format == vk::Format::B8G8R8A8_SRGB
                    && surface_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .ok_or(RenderError::MissingSurfaceFormat.into())
    }

    fn get_extent(
        window: &Window,
        surface_capabilities: vk::SurfaceCapabilitiesKHR,
    ) -> vk::Extent2D {
        if surface_capabilities.current_extent.width != u32::MAX {
            surface_capabilities.current_extent
        } else {
            vk::Extent2D::builder()
                .width(window.inner_size().width.clamp(
                    surface_capabilities.min_image_extent.width,
                    surface_capabilities.max_image_extent.width,
                ))
                .height(window.inner_size().height.clamp(
                    surface_capabilities.min_image_extent.height,
                    surface_capabilities.max_image_extent.height,
                ))
                .build()
        }
    }

    fn get_present_mode(
        present_modes: &[vk::PresentModeKHR],
        preferred_present_modes: &[vk::PresentModeKHR],
    ) -> NeuclidioResult<vk::PresentModeKHR> {
        if present_modes.is_empty() {
            return Err(RenderError::MissingPresentMode.into());
        }

        for preferred_present_mode in preferred_present_modes.iter() {
            if present_modes.contains(preferred_present_mode) {
                return Ok(*preferred_present_mode);
            }
        }

        Ok(present_modes[0])
    }

    fn create_image_views(
        neuclidio_window: &NeuclidioWindow,
        images: &[vk::Image],
        image_format: vk::Format,
    ) -> NeuclidioResult<Vec<vk::ImageView>> {
        let mut image_views = Vec::with_capacity(images.len());

        for image in images.iter() {
            let components = vk::ComponentMapping::builder()
                .r(vk::ComponentSwizzle::IDENTITY)
                .g(vk::ComponentSwizzle::IDENTITY)
                .b(vk::ComponentSwizzle::IDENTITY)
                .a(vk::ComponentSwizzle::IDENTITY);

            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            let image_view_create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::_2D)
                .format(image_format)
                .components(components)
                .subresource_range(subresource_range);

            let image_view = unsafe {
                neuclidio_window
                    .logical_device
                    .create_image_view(&image_view_create_info, None)?
            };

            image_views.push(image_view);
        }

        Ok(image_views)
    }
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
            return Err(RenderError::MissingSwapChainSupport.into());
        }

        Ok(Self {
            capabilities,
            formats,
            present_modes,
        })
    }
}
