use crate::engine::render::error::NeuclidioRenderError;
use crate::engine::render::pipeline::NeuclidioRenderPipeline;
use crate::engine::render::pipeline::standard::NeuclidioStandardRenderPipeline;
use crate::engine::render::windowing::device_extension_support::DeviceExtensionSupport;
use crate::engine::render::windowing::queue_family_indices::QueueFamilyIndices;
use crate::engine::render::windowing::swap_chain::{SwapChain, SwapChainSupport};
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::{NeuclidioError, NeuclidioResult};
use itertools::Itertools;
use log::{debug, info, trace, warn};
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, c_void};
use vulkanalia::vk::{
    DeviceV1_0, EntryV1_0, ExtDebugUtilsExtensionInstanceCommands, HasBuilder, InstanceV1_0,
};
use vulkanalia::{Device, window as vk_window};
use vulkanalia::{Entry, Instance, Version, vk};
use winit::window::{Window, WindowId};

/// Whether the validation layers should be enabled.
const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

/// The name of the validation layers.
const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

/// The required device extensions.
const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

/// The Vulkan SDK version that started requiring the portability subset extension for macOS.
const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

pub struct NeuclidioRenderEngineWindowManager {
    application_info: vk::ApplicationInfo,
    vulkan_entry: Entry,
    windows: HashMap<WindowId, NeuclidioWindow>,
    pipelines: HashMap<WindowId, Box<dyn NeuclidioRenderPipeline>>,
}

impl NeuclidioRenderEngineWindowManager {
    pub fn new(application_info: vk::ApplicationInfo, vulkan_entry: Entry) -> Self {
        Self {
            application_info,
            vulkan_entry,
            windows: HashMap::new(),
            pipelines: HashMap::new(),
        }
    }

    pub fn prepare_for_window(&mut self, window: &Window) -> NeuclidioResult<()> {
        let window_id = window.id();

        debug!("Creating Vulkan instance for window with id: {window_id:?}");
        let (instance, debug_messenger) = self.create_instance(window)?;

        debug!("Creating Vulkan surface for window with id: {window_id:?}");
        let surface = Self::create_surface(&instance, window)?;

        debug!("Picking Vulkan physical device for window with id: {window_id:?}");
        let (physical_device, queue_family_indices, swap_chain_support) =
            Self::pick_physical_device(&instance, surface)?;

        debug!("Creating Vulkan logical device for window with id: {window_id:?}");
        let logical_device =
            self.create_logical_device(&instance, physical_device, queue_family_indices)?;

        let graphics_queue = Self::get_device_queue(&logical_device, queue_family_indices.graphics);
        let present_queue = Self::get_device_queue(&logical_device, queue_family_indices.present);

        let mut neuclidio_window = NeuclidioWindow {
            id: window.id(),
            instance,
            logical_device,
            queue_family_indices,
            swap_chain_support,
            surface,
            graphics_queue,
            present_queue,
            #[cfg(debug_assertions)]
            debug_messenger: debug_messenger.unwrap(),
            swap_chain: None,
        };

        debug!("Creating Vulkan swap chain for window with id: {window_id:?}");
        let swap_chain = SwapChain::new(
            window,
            &neuclidio_window,
            2,                                                        // TODO: Make configurable
            &[vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::FIFO], // TODO: Make configurable
        )?;

        neuclidio_window.swap_chain.replace(swap_chain);

        let pipeline = Box::new(NeuclidioStandardRenderPipeline::new(
            &neuclidio_window,
            physical_device,
            3, // TODO: Make this configurable
        )?);

        self.windows.insert(window_id, neuclidio_window);
        self.pipelines.insert(window_id, pipeline);

        Ok(())
    }

    pub fn render_on_window(&mut self, window: &Window) -> NeuclidioResult<()> {
        let window_id = window.id();
        let neuclidio_window = match self.windows.get(&window_id) {
            Some(window_data) => window_data,
            None => {
                debug!("Tried to render without Vulkan prepared for window with id: {window_id:?}");
                return Ok(());
            }
        };
        let pipeline = match self.pipelines.get_mut(&window_id) {
            Some(pipeline) => pipeline,
            None => {
                debug!("Tried to render without Vulkan prepared for window with id: {window_id:?}");
                return Ok(());
            }
        };

        if let Some(true) = window.is_minimized() {
            return Ok(());
        }

        match pipeline.render(neuclidio_window) {
            Ok(_) => {}
            Err(NeuclidioError::RenderError(NeuclidioRenderError::OutOfDateSwapChain)) => {
                self.handle_window_change(window)?;
                return Ok(());
            }
            Err(err) => return Err(err),
        }

        Ok(())
    }

    pub fn handle_window_change(&mut self, window: &Window) -> NeuclidioResult<()> {
        let window_id = window.id();

        let neuclidio_window = match self.windows.get_mut(&window_id) {
            Some(window_data) => window_data,
            None => {
                debug!(
                    "Tried to handle window change without Vulkan prepared for window with id: {window_id:?}"
                );
                return Ok(());
            }
        };

        unsafe { neuclidio_window.logical_device.device_wait_idle()? };

        let mut pipeline = self.pipelines.get_mut(&window_id);

        if let Some(pipeline) = pipeline.as_ref() {
            pipeline.prepare_for_reset(neuclidio_window);
        }

        if let Some(swap_chain) = neuclidio_window.swap_chain.take() {
            swap_chain.destroy(neuclidio_window);
        }

        let swap_chain = SwapChain::new(
            window,
            neuclidio_window,
            2,                                                        // TODO: Make configurable
            &[vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::FIFO], // TODO: Make configurable
        )?;

        neuclidio_window.swap_chain.replace(swap_chain);

        if let Some(pipeline) = pipeline.as_mut() {
            pipeline.reset(neuclidio_window)?;
        }

        Ok(())
    }

    pub fn cleanup_for_window(&mut self, window_id: WindowId) -> NeuclidioResult<()> {
        let neuclidio_window = match self.windows.get_mut(&window_id) {
            Some(window_data) => window_data,
            None => {
                debug!(
                    "Tried to destroy pipeline without Vulkan prepared for window with id: {window_id:?}"
                );
                return Ok(());
            }
        };

        unsafe { neuclidio_window.logical_device.device_wait_idle()? };

        if let Some(pipeline) = self.pipelines.remove(&window_id) {
            pipeline.prepare_for_reset(neuclidio_window);

            if let Some(swap_chain) = neuclidio_window.swap_chain.take() {
                swap_chain.destroy(neuclidio_window);
            }

            pipeline.destroy(neuclidio_window);
        }

        self.windows.remove(&window_id);

        Ok(())
    }

    fn create_instance(
        &self,
        window: &Window,
    ) -> NeuclidioResult<(Instance, Option<vk::DebugUtilsMessengerEXT>)> {
        let available_instance_layers: HashSet<_> = unsafe {
            self.vulkan_entry
                .enumerate_instance_layer_properties()?
                .iter()
                .map(|layer| layer.layer_name)
                .collect()
        };

        if VALIDATION_ENABLED && !available_instance_layers.contains(&VALIDATION_LAYER) {
            return Err(NeuclidioRenderError::MissingValidationLayer.into());
        }

        let layers = if VALIDATION_ENABLED {
            vec![VALIDATION_LAYER.as_ptr()]
        } else {
            vec![]
        };

        let mut extensions = vk_window::get_required_instance_extensions(window)
            .iter()
            .map(|e| e.as_ptr())
            .collect::<Vec<_>>();

        // Required by Vulkan SDK on macOS since 1.3.216.
        let flags = if cfg!(target_os = "macos")
            && self.vulkan_entry.version()? >= PORTABILITY_MACOS_VERSION
        {
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

        let instance = unsafe {
            self.vulkan_entry
                .create_instance(&instance_create_info_builder.build(), None)?
        };
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
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        queue_family_indices: QueueFamilyIndices,
    ) -> NeuclidioResult<Device> {
        let queue_infos: Vec<_> = queue_family_indices
            .unique()
            .iter()
            .map(|i| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(*i)
                    .queue_priorities(&[1.0]) // Maximum priority
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
        if cfg!(target_os = "macos") && self.vulkan_entry.version()? >= PORTABILITY_MACOS_VERSION {
            extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
        }

        let features = vk::PhysicalDeviceFeatures::builder().build();

        let logical_device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_infos)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .enabled_features(&features)
            .build();

        let logical_device =
            unsafe { instance.create_device(physical_device, &logical_device_create_info, None)? };

        Ok(logical_device)
    }

    fn create_surface(instance: &Instance, window: &Window) -> NeuclidioResult<vk::SurfaceKHR> {
        let surface = unsafe { vk_window::create_surface(instance, window, window)? };
        Ok(surface)
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
                    Ok(physical_devices_capabilities) => {
                        viable_physical_devices.push((
                            (
                                physical_device,
                                physical_devices_capabilities.0,
                                physical_devices_capabilities.1,
                            ),
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
            Err(NeuclidioRenderError::NoSuitableDevice.into())
        }
    }

    fn check_physical_device(
        instance: &Instance,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> NeuclidioResult<(QueueFamilyIndices, SwapChainSupport)> {
        let queue_family_indices = QueueFamilyIndices::new(instance, surface, physical_device)?;
        DeviceExtensionSupport::new(instance, physical_device, DEVICE_EXTENSIONS)?;
        let swap_chain_support = SwapChainSupport::new(instance, surface, physical_device)?;

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
