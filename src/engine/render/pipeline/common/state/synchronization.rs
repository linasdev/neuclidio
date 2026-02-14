use crate::engine::render::vulkan_context::VulkanContext;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use log::debug;
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, DeviceV1_2, HasBuilder};
use winit::window::WindowId;

pub struct RenderPipelineSynchronizationState {
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    frame_index_semaphore: vk::Semaphore,
    frame_index: u64,
    frame_in_flight_index: usize,
    max_frames_in_flight: usize,
}

impl RenderPipelineSynchronizationState {
    pub fn new(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        debug!(
            "Creating Vulkan synchronization objects for window with id: {:?}",
            neuclidio_window.id
        );

        let frame_index_semaphore = Self::create_timeline_semaphore(vulkan_context)?;
        let mut image_available_semaphores = Vec::with_capacity(max_frames_in_flight);
        let mut render_finished_semaphores = Vec::with_capacity(max_frames_in_flight);

        for _ in 0..max_frames_in_flight {
            image_available_semaphores.push(Self::create_semaphore(vulkan_context)?);
            render_finished_semaphores.push(Self::create_semaphore(vulkan_context)?);
        }

        let synchronization = Self {
            image_available_semaphores,
            render_finished_semaphores,
            frame_index_semaphore,
            frame_index: 0,
            frame_in_flight_index: 0,
            max_frames_in_flight,
        };

        Ok(synchronization)
    }

    pub fn destroy(self, vulkan_context: &VulkanContext, window_id: WindowId) {
        let logical_device = &vulkan_context.logical_device;

        debug!("Destroying Vulkan semaphores for window with id: {window_id:?}");

        unsafe {
            logical_device.destroy_semaphore(self.frame_index_semaphore, None);
        }

        for semaphore in self.render_finished_semaphores.iter() {
            unsafe {
                logical_device.destroy_semaphore(*semaphore, None);
            }
        }
        for semaphore in self.image_available_semaphores.iter() {
            unsafe {
                logical_device.destroy_semaphore(*semaphore, None);
            }
        }
    }

    pub fn current_image_available_semaphore(&self) -> vk::Semaphore {
        self.image_available_semaphores[self.frame_in_flight_index]
    }

    pub fn current_render_finished_semaphore(&self) -> vk::Semaphore {
        self.render_finished_semaphores[self.frame_in_flight_index]
    }

    pub fn frame_index_semaphore(&self) -> vk::Semaphore {
        self.frame_index_semaphore
    }

    pub fn frame_index_semaphore_value(
        &self,
        vulkan_context: &VulkanContext,
    ) -> NeuclidioResult<u64> {
        let frame_index_semaphore_value = unsafe {
            vulkan_context
                .logical_device
                .get_semaphore_counter_value(self.frame_index_semaphore)?
        };

        Ok(frame_index_semaphore_value)
    }

    pub fn frame_index(&self) -> u64 {
        self.frame_index
    }

    pub fn frame_index_semaphore_required_value(&self) -> u64 {
        if self.frame_index < self.max_frames_in_flight as u64 {
            return 0;
        }

        self.frame_index - self.max_frames_in_flight as u64
    }

    pub fn wait_for_frame_index_semaphore_value(
        &self,
        vulkan_context: &VulkanContext,
    ) -> NeuclidioResult<()> {
        if self.frame_index < self.max_frames_in_flight as u64 {
            return Ok(());
        }

        let semaphores = [self.frame_index_semaphore];
        let values = [self.frame_index_semaphore_required_value()];
        let semaphore_wait_info = vk::SemaphoreWaitInfo::builder()
            .semaphores(&semaphores)
            .values(&values)
            .build();

        unsafe {
            vulkan_context
                .logical_device
                .wait_semaphores(&semaphore_wait_info, 0)?;
        }

        Ok(())
    }

    pub fn increment_frame(&mut self) {
        self.frame_in_flight_index = (self.frame_in_flight_index + 1) % self.max_frames_in_flight;
        self.frame_index += 1;
    }

    fn create_semaphore(vulkan_context: &VulkanContext) -> NeuclidioResult<vk::Semaphore> {
        let semaphore_create_info = vk::SemaphoreCreateInfo::builder().build();

        let semaphore = unsafe {
            vulkan_context
                .logical_device
                .create_semaphore(&semaphore_create_info, None)?
        };

        Ok(semaphore)
    }

    fn create_timeline_semaphore(vulkan_context: &VulkanContext) -> NeuclidioResult<vk::Semaphore> {
        let mut semaphore_type_create_info = vk::SemaphoreTypeCreateInfo::builder()
            .semaphore_type(vk::SemaphoreType::TIMELINE)
            .initial_value(0)
            .build();

        let semaphore_create_info = vk::SemaphoreCreateInfo::builder()
            .push_next(&mut semaphore_type_create_info)
            .build();

        let semaphore = unsafe {
            vulkan_context
                .logical_device
                .create_semaphore(&semaphore_create_info, None)?
        };

        Ok(semaphore)
    }
}
