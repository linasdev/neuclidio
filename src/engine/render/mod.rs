use crate::engine::render::error::RenderError;
use crate::engine::render::pipeline::standard::StandardRenderPipeline;
use crate::engine::render::pipeline::{RenderPipeline, RenderPipelineExt};
use crate::engine::render::renderable::Renderable;
use crate::engine::render::vulkan_context::VulkanContext;
use crate::engine::render::windowing::device_extension_support::DeviceExtensionSupport;
use crate::engine::render::windowing::queue_family_indices::QueueFamilyIndices;
use crate::engine::render::windowing::swap_chain::{SwapChain, SwapChainSupport};
use crate::engine::render::windowing::synchronization_support::SynchronizationSupport;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::entity::Entity;
use crate::error::{NeuclidioError, NeuclidioResult};
use itertools::Itertools;
use log::{debug, info, trace, warn};
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, c_void};
use vulkanalia::loader::{LIBRARY, LibloadingLoader};
use vulkanalia::vk::{
    DeviceV1_0, EntryV1_0, ExtDebugUtilsExtensionInstanceCommands, HasBuilder, InstanceV1_0,
};
use vulkanalia::window as vk_window;
use vulkanalia::{Device, Entry, Instance, Version, vk};
use vulkanalia_vma::{Allocator, AllocatorOptions};
use winit::window::{Window, WindowId};

pub mod builder;
pub mod error;
pub mod pipeline;

pub(crate) mod renderable;
pub(crate) mod vulkan_context;
pub(crate) mod windowing;

/// Whether the validation layers should be enabled.
const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

/// The name of the validation layers.
const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

/// The required device extensions.
const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

/// The Vulkan SDK version that started requiring the portability subset extension for macOS.
const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

/// The Vulkan SDK version that included synchronization2 into the core specification
const SYNCHRONIZATION2_VULKAN_VERSION: Version = Version::new(1, 3, 0);

pub struct RenderEngine {
    application_info: vk::ApplicationInfo,
    vulkan_context: Option<VulkanContext>,
    render_pipeline: Option<RenderPipeline>,
    windows: HashMap<WindowId, NeuclidioWindow>,
}

impl RenderEngine {
    pub fn new(application_info: vk::ApplicationInfo) -> Self {
        Self {
            application_info,
            vulkan_context: None,
            render_pipeline: None,
            windows: HashMap::new(),
        }
    }

    pub fn prepare_for_window(&mut self, window: &Window) -> NeuclidioResult<()> {
        let window_id = window.id();

        let surface = if let Some(vulkan_context) = self.vulkan_context.as_ref() {
            debug!("Creating Vulkan surface for window with id: {window_id:?}");
            Self::create_surface(&vulkan_context.instance, window)?
        } else {
            self.prepare_vulkan(window)?
        };

        let neuclidio_window = NeuclidioWindow {
            id: window_id,
            surface,
            swap_chain: None,
        };

        self.windows.insert(window_id, neuclidio_window);

        Ok(())
    }

    pub fn render_on_window(&mut self, window: &Window) -> NeuclidioResult<()> {
        if let Some(true) = window.is_minimized() {
            return Ok(());
        }

        if let (Some(vulkan_context), Some(render_pipeline), Some(neuclidio_window)) = (
            self.vulkan_context.as_ref(),
            self.render_pipeline.as_mut(),
            self.windows.get(&window.id()),
        ) {
            match render_pipeline.render(vulkan_context, neuclidio_window) {
                Ok(_) => {}
                Err(NeuclidioError::RenderError(RenderError::OutOfDateSwapChain)) => {
                    self.handle_window_change(window)?;
                    return Ok(());
                }
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }

    pub fn handle_window_change(&mut self, window: &Window) -> NeuclidioResult<()> {
        if let (Some(vulkan_context), Some(neuclidio_window)) = (
            self.vulkan_context.as_ref(),
            self.windows.get_mut(&window.id()),
        ) {
            unsafe { vulkan_context.logical_device.device_wait_idle()? };

            if let Some(render_pipeline) = self.render_pipeline.as_mut() {
                render_pipeline.prepare_for_window_reset(vulkan_context, neuclidio_window);
            }

            if let Some(swap_chain) = neuclidio_window.swap_chain.take() {
                swap_chain.destroy(vulkan_context);
            }

            let swap_chain = SwapChain::new(
                window,
                vulkan_context,
                neuclidio_window,
                2,                                                        // TODO: Make configurable
                &[vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::FIFO], // TODO: Make configurable
            )?;

            neuclidio_window.swap_chain.replace(swap_chain);

            if let Some(render_pipeline) = self.render_pipeline.as_mut() {
                render_pipeline.reset_window(vulkan_context, neuclidio_window)?;
            }
        }

        Ok(())
    }

    pub fn clean_up_for_window(&mut self, window_id: WindowId) -> NeuclidioResult<()> {
        if let (Some(vulkan_context), Some(neuclidio_window)) = (
            self.vulkan_context.as_ref(),
            self.windows.get_mut(&window_id),
        ) {
            unsafe { vulkan_context.logical_device.device_wait_idle()? };

            if let Some(render_pipeline) = self.render_pipeline.as_mut() {
                render_pipeline.clean_up_for_window(vulkan_context, neuclidio_window);
            }

            if let Some(swap_chain) = neuclidio_window.swap_chain.take() {
                swap_chain.destroy(vulkan_context);
            }

            if let Some(neuclidio_window) = self.windows.remove(&window_id) {
                neuclidio_window.destroy(vulkan_context);
            }
        }

        Ok(())
    }

    pub fn submit_entity(&mut self, window_id: WindowId, entity: &Entity) -> NeuclidioResult<()> {
        if let (Some(vulkan_context), Some(render_pipeline), Some(neuclidio_window)) = (
            self.vulkan_context.as_ref(),
            self.render_pipeline.as_mut(),
            self.windows.get(&window_id),
        ) {
            render_pipeline.submit_entity(vulkan_context, neuclidio_window, entity)?;
        }

        Ok(())
    }

    pub fn remove_entity(&mut self, entity: &Entity) -> NeuclidioResult<()> {
        if let Some(render_pipeline) = self.render_pipeline.as_mut() {
            render_pipeline.remove_entity(entity)?;
        }

        Ok(())
    }

    pub fn handle_renderable_added(
        &mut self,
        entity: &Entity,
        renderable: Renderable,
    ) -> NeuclidioResult<()> {
        if let (Some(vulkan_context), Some(render_pipeline)) =
            (self.vulkan_context.as_ref(), self.render_pipeline.as_mut())
        {
            render_pipeline.handle_renderable_added(vulkan_context, entity, renderable)?;
        }

        Ok(())
    }

    pub fn handle_renderable_removed(
        &mut self,
        entity: &Entity,
        renderable: Renderable,
    ) -> NeuclidioResult<()> {
        if let Some(render_pipeline) = self.render_pipeline.as_mut() {
            render_pipeline.handle_renderable_removed(entity, renderable)?;
        }

        Ok(())
    }

    fn prepare_vulkan(&mut self, window: &Window) -> NeuclidioResult<vk::SurfaceKHR> {
        let window_id = window.id();

        let vulkan_entry = unsafe {
            let loader = LibloadingLoader::new(LIBRARY)?;
            Entry::new(loader)?
        };

        debug!("Creating Vulkan instance");
        #[cfg(debug_assertions)]
        let (instance, debug_messenger) = self.create_instance(&vulkan_entry, window)?;

        #[cfg(not(debug_assertions))]
        let (instance, _) = self.create_instance(window)?;

        debug!("Creating Vulkan surface for window with id: {window_id:?}");
        let surface = Self::create_surface(&instance, window)?;

        debug!("Picking Vulkan physical device");
        let (physical_device, queue_family_indices, swap_chain_support) =
            Self::pick_physical_device(&instance, surface)?;

        debug!("Creating Vulkan logical device");
        let logical_device = self.create_logical_device(
            &vulkan_entry,
            &instance,
            physical_device,
            queue_family_indices,
        )?;

        debug!("Creating Vulkan Memory Allocator");

        let allocator_options = AllocatorOptions::new(&instance, &logical_device, physical_device);
        let allocator = unsafe { Allocator::new(&allocator_options)? };

        let surface_format = Self::get_surface_format(&swap_chain_support.formats)?;
        let graphics_queue = Self::get_device_queue(&logical_device, queue_family_indices.graphics);
        let present_queue = Self::get_device_queue(&logical_device, queue_family_indices.present);
        let transfer_queue = Self::get_device_queue(&logical_device, queue_family_indices.transfer);

        let vulkan_context = VulkanContext {
            vulkan_entry,
            instance,
            logical_device,
            allocator,
            queue_family_indices,
            physical_device,
            graphics_queue,
            present_queue,
            transfer_queue,
            surface_format,
            #[cfg(debug_assertions)]
            debug_messenger: debug_messenger.unwrap(),
        };

        let render_pipeline = RenderPipeline::Standard(Box::new(StandardRenderPipeline::new(
            &vulkan_context,
            2, // TODO: Make this configurable
        )?));

        self.vulkan_context.replace(vulkan_context);
        self.render_pipeline.replace(render_pipeline);

        Ok(surface)
    }

    fn create_instance(
        &self,
        vulkan_entry: &Entry,
        window: &Window,
    ) -> NeuclidioResult<(Instance, Option<vk::DebugUtilsMessengerEXT>)> {
        let vulkan_version = vulkan_entry.version()?;

        let available_instance_layers: HashSet<_> = unsafe {
            vulkan_entry
                .enumerate_instance_layer_properties()?
                .iter()
                .map(|layer| layer.layer_name)
                .collect()
        };

        if VALIDATION_ENABLED && !available_instance_layers.contains(&VALIDATION_LAYER) {
            return Err(RenderError::MissingValidationLayer.into());
        }

        let layers = if VALIDATION_ENABLED {
            vec![VALIDATION_LAYER.as_ptr()]
        } else {
            vec![]
        };

        let mut extensions: Vec<_> = vk_window::get_required_instance_extensions(window)
            .iter()
            .map(|e| e.as_ptr())
            .collect();

        // Required by Vulkan SDK on macOS since 1.3.216.
        let flags = if cfg!(target_os = "macos") && vulkan_version >= PORTABILITY_MACOS_VERSION {
            info!("Enabling extensions for macOS portability");
            extensions.push(
                vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION
                    .name
                    .as_ptr(),
            );
            extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::empty()
        };

        if vulkan_version < SYNCHRONIZATION2_VULKAN_VERSION {
            extensions.push(vk::KHR_SYNCHRONIZATION2_EXTENSION.name.as_ptr());
        }

        if VALIDATION_ENABLED {
            extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
        }

        let mut instance_create_info_builder = vk::InstanceCreateInfo::builder()
            .application_info(&self.application_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .flags(flags);

        let mut debug_messenger_create_info = None;

        if VALIDATION_ENABLED {
            debug_messenger_create_info.replace(
                vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                    )
                    .user_callback(Some(debug_callback))
                    .build(),
            );

            instance_create_info_builder = instance_create_info_builder
                .push_next(debug_messenger_create_info.as_mut().unwrap());
        }

        let instance =
            unsafe { vulkan_entry.create_instance(&instance_create_info_builder.build(), None)? };
        let debug_messenger = if let Some(debug_messenger_create_info) = debug_messenger_create_info
        {
            unsafe {
                Some(
                    instance
                        .create_debug_utils_messenger_ext(&debug_messenger_create_info, None)?,
                )
            }
        } else {
            None
        };

        Ok((instance, debug_messenger))
    }

    fn create_logical_device(
        &self,
        vulkan_entry: &Entry,
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        queue_family_indices: QueueFamilyIndices,
    ) -> NeuclidioResult<Device> {
        let physical_device_vulkan_version = unsafe {
            Version::from(
                instance
                    .get_physical_device_properties(physical_device)
                    .api_version,
            )
        };
        let vulkan_version = vulkan_entry.version()?.min(physical_device_vulkan_version);

        let queue_priorities = [1.0];
        let queue_infos: Vec<_> = queue_family_indices
            .unique()
            .iter()
            .map(|i| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(*i)
                    .queue_priorities(&queue_priorities) // Maximum priority
                    .build()
            })
            .collect();

        let layers = if VALIDATION_ENABLED {
            vec![VALIDATION_LAYER.as_ptr()]
        } else {
            vec![]
        };

        let mut extensions: Vec<_> = DEVICE_EXTENSIONS
            .iter()
            .map(|extension| extension.as_ptr())
            .collect();

        // Required by Vulkan SDK on macOS since 1.3.216.
        if cfg!(target_os = "macos") && vulkan_version >= PORTABILITY_MACOS_VERSION {
            extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
        }

        if vulkan_version < SYNCHRONIZATION2_VULKAN_VERSION {
            extensions.push(vk::KHR_SYNCHRONIZATION2_EXTENSION.name.as_ptr());
        }

        let mut timeline_semaphore_features =
            vk::PhysicalDeviceTimelineSemaphoreFeatures::builder()
                .timeline_semaphore(true)
                .build();

        let mut synchronization2_features = vk::PhysicalDeviceSynchronization2Features::builder()
            .synchronization2(true)
            .build();

        let features = vk::PhysicalDeviceFeatures::builder().build();
        let mut features2 = vk::PhysicalDeviceFeatures2::builder()
            .push_next(&mut timeline_semaphore_features)
            .push_next(&mut synchronization2_features)
            .features(features)
            .build();

        let logical_device_create_info = vk::DeviceCreateInfo::builder()
            .push_next(&mut features2)
            .queue_create_infos(&queue_infos)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .build();

        let logical_device =
            unsafe { instance.create_device(physical_device, &logical_device_create_info, None)? };

        Ok(logical_device)
    }

    fn create_surface(instance: &Instance, window: &Window) -> NeuclidioResult<vk::SurfaceKHR> {
        let surface = unsafe { vk_window::create_surface(instance, window, window)? };
        Ok(surface)
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

    fn pick_physical_device(
        instance: &Instance,
        surface: vk::SurfaceKHR,
    ) -> NeuclidioResult<(vk::PhysicalDevice, QueueFamilyIndices, SwapChainSupport)> {
        let mut viable_physical_devices = vec![];

        unsafe {
            for physical_device in instance.enumerate_physical_devices()? {
                let properties = instance.get_physical_device_properties(physical_device);

                match Self::check_physical_device(instance, surface, physical_device) {
                    Err(err) => {
                        warn!(
                            "Skipping physical device (`{}`): {:?}",
                            properties.device_name, err
                        );
                    }
                    Ok((queue_family_indices, swap_chain_support)) => {
                        viable_physical_devices.push((
                            (physical_device, queue_family_indices, swap_chain_support),
                            properties,
                        ));
                    }
                }
            }
        }

        let physical_device_pair = viable_physical_devices
            .into_iter()
            .map(|physical_device_pair| {
                (
                    Self::rate_physical_device(physical_device_pair.1),
                    physical_device_pair,
                )
            })
            .sorted_by(|a, b| Ord::cmp(&b.0, &a.0))
            .next()
            .map(|rated_physical_device| rated_physical_device.1);

        if let Some(physical_device_pair) = physical_device_pair {
            info!(
                "Picked physical device ('{}')",
                physical_device_pair.1.device_name
            );
            Ok(physical_device_pair.0)
        } else {
            Err(RenderError::NoSuitableDevice.into())
        }
    }

    fn check_physical_device(
        instance: &Instance,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> NeuclidioResult<(QueueFamilyIndices, SwapChainSupport)> {
        let queue_family_indices = QueueFamilyIndices::new(instance, surface, physical_device)?;
        let swap_chain_support = SwapChainSupport::new(instance, surface, physical_device)?;
        DeviceExtensionSupport::new(instance, physical_device, DEVICE_EXTENSIONS)?;
        SynchronizationSupport::new(instance, physical_device)?;

        Ok((queue_family_indices, swap_chain_support))
    }

    fn rate_physical_device(physical_device_properties: vk::PhysicalDeviceProperties) -> u32 {
        let mut score = 0;

        if physical_device_properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            score += 1000;
        } else if physical_device_properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU {
            score += 100;
        }

        // TODO: Implement this further

        score
    }

    fn get_device_queue(logical_device: &Device, index: u32) -> vk::Queue {
        unsafe { logical_device.get_device_queue(index, 0) }
    }
}

impl Drop for RenderEngine {
    fn drop(&mut self) {
        if let Some(vulkan_context) = self.vulkan_context.take() {
            if let Some(render_pipeline) = self.render_pipeline.take() {
                render_pipeline.destroy(&vulkan_context);
            }

            vulkan_context.destroy();
        }
    }
}

extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();
    trace!("Vulkan debug message ({severity:?}): ({type_:?}) {message}");

    vk::FALSE
}
