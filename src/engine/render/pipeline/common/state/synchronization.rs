use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::vulkan_context::VulkanContext;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use log::debug;
use std::collections::HashMap;
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, DeviceV1_2, HasBuilder};
use winit::window::WindowId;

pub struct RenderPipelineSynchronizationState {
    window_states: HashMap<WindowId, RenderPipelineSynchronizationWindowState>,
    max_frames_in_flight: usize,
}

impl RenderPipelineSynchronizationState {
    pub fn new(max_frames_in_flight: usize) -> NeuclidioResult<Self> {
        let window_states = HashMap::new();

        let synchronization = Self {
            window_states,
            max_frames_in_flight,
        };

        Ok(synchronization)
    }

    pub fn prepare_for_window_reset(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) {
        if let Some(window_state) = self.window_states.get_mut(&neuclidio_window.id) {
            window_state.prepare_for_window_reset(vulkan_context);
        }
    }

    pub fn reset_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<()> {
        if let Some(window_state) = self.window_states.get_mut(&neuclidio_window.id) {
            window_state.reset_window(vulkan_context, neuclidio_window)?;
            return Ok(());
        }

        let mut window_state = RenderPipelineSynchronizationWindowState::new(
            vulkan_context,
            neuclidio_window,
            self.max_frames_in_flight,
        )?;
        window_state.reset_window(vulkan_context, neuclidio_window)?;

        self.window_states.insert(neuclidio_window.id, window_state);

        Ok(())
    }

    pub fn clean_up_for_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) {
        if let Some(window_state) = self.window_states.remove(&neuclidio_window.id) {
            window_state.destroy(vulkan_context);
        }
    }

    pub fn destroy(self, vulkan_context: &VulkanContext) {
        for window_state in self.window_states.into_values() {
            window_state.destroy(vulkan_context);
        }
    }

    pub fn current_image_available_semaphore(
        &self,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<vk::Semaphore> {
        self.window_states
            .get(&neuclidio_window.id)
            .map(|window_state| {
                window_state.image_available_semaphores[window_state.frame_in_flight_index]
            })
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    pub fn current_render_finished_semaphore(
        &self,
        neuclidio_window: &NeuclidioWindow,
        image_index: usize,
    ) -> NeuclidioResult<vk::Semaphore> {
        self.window_states
            .get(&neuclidio_window.id)
            .and_then(|window_state| {
                if let Some(render_finished_semaphores) =
                    window_state.render_finished_semaphores.as_ref()
                {
                    Some(render_finished_semaphores[image_index])
                } else {
                    None
                }
            })
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    pub fn frame_index_semaphore(
        &self,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<vk::Semaphore> {
        self.window_states
            .get(&neuclidio_window.id)
            .map(|window_state| window_state.frame_index_semaphore)
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    // TODO:REMOVE
    pub fn finished_frame_index_by_window_id(
        &self,
        vulkan_context: &VulkanContext,
        window_id: WindowId,
    ) -> NeuclidioResult<u64> {
        let frame_index_semaphore = self
            .window_states
            .get(&window_id)
            .ok_or(RenderPipelineError::Unprepared)?
            .frame_index_semaphore;
        let frame_index_semaphore_value = unsafe {
            vulkan_context
                .logical_device
                .get_semaphore_counter_value(frame_index_semaphore)?
        };

        Ok(frame_index_semaphore_value / 2)
    }

    pub fn finished_frame_index(
        &self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<u64> {
        let frame_index_semaphore = self.frame_index_semaphore(neuclidio_window)?;
        let frame_index_semaphore_value = unsafe {
            vulkan_context
                .logical_device
                .get_semaphore_counter_value(frame_index_semaphore)?
        };

        Ok(frame_index_semaphore_value)
    }

    pub fn frame_index(&self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<u64> {
        self.window_states
            .get(&neuclidio_window.id)
            .map(|window_state| window_state.frame_index)
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    pub fn frame_in_flight_index(
        &self,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<usize> {
        self.window_states
            .get(&neuclidio_window.id)
            .map(|window_state| window_state.frame_in_flight_index)
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    pub fn frame_index_semaphore_required_value(
        &self,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<u64> {
        let frame_index = self.frame_index(neuclidio_window)?;
        if frame_index < self.max_frames_in_flight as u64 {
            Ok(0)
        } else {
            Ok((frame_index - self.max_frames_in_flight as u64 + 1) * 2)
        }
    }

    pub fn frame_index_semaphore_first_value(
        &self,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<u64> {
        let frame_index = self.frame_index(neuclidio_window)?;
        Ok(frame_index * 2 + 1)
    }

    pub fn frame_index_semaphore_second_value(
        &self,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<u64> {
        let frame_index = self.frame_index(neuclidio_window)? + 1;
        Ok(frame_index * 2)
    }

    pub fn wait_for_frame_index_semaphore_value(
        &self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<()> {
        let frame_index_semaphore = self.frame_index_semaphore(neuclidio_window)?;
        let frame_index_semaphore_required_value =
            self.frame_index_semaphore_required_value(neuclidio_window)?;

        if frame_index_semaphore_required_value == 0 {
            return Ok(());
        }

        let semaphores = [frame_index_semaphore];
        let values = [frame_index_semaphore_required_value];
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

    pub fn increment_frame(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()> {
        self.window_states
            .get_mut(&neuclidio_window.id)
            .map(|window_state| {
                window_state.frame_in_flight_index =
                    (window_state.frame_in_flight_index + 1) % self.max_frames_in_flight;
                window_state.frame_index += 1;
                ()
            })
            .ok_or(RenderPipelineError::Unprepared.into())
    }
}

pub struct RenderPipelineSynchronizationWindowState {
    window_id: WindowId,
    frame_index_semaphore: vk::Semaphore,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Option<Vec<vk::Semaphore>>,
    frame_index: u64,
    frame_in_flight_index: usize,
}

impl RenderPipelineSynchronizationWindowState {
    pub fn new(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        let window_id = neuclidio_window.id;

        debug!("Creating Vulkan timeline semaphore for window with id: {window_id:?}");

        let frame_index_semaphore = Self::create_timeline_semaphore(vulkan_context)?;

        debug!("Creating Vulkan image available semaphores for window with id: {window_id:?}");

        let mut image_available_semaphores = Vec::with_capacity(max_frames_in_flight);

        for _ in 0..max_frames_in_flight {
            image_available_semaphores.push(Self::create_semaphore(vulkan_context)?);
        }

        let synchronization = Self {
            window_id,
            frame_index_semaphore,
            image_available_semaphores,
            render_finished_semaphores: None,
            frame_index: 0,
            frame_in_flight_index: 0,
        };

        Ok(synchronization)
    }

    pub fn prepare_for_window_reset(&mut self, vulkan_context: &VulkanContext) {
        let logical_device = &vulkan_context.logical_device;

        if let Some(render_finished_semaphores) = self.render_finished_semaphores.take() {
            debug!(
                "Destroying Vulkan render finished semaphores for window with id: {:?}",
                self.window_id
            );

            for semaphore in render_finished_semaphores.iter() {
                unsafe {
                    logical_device.destroy_semaphore(*semaphore, None);
                }
            }
        }
    }

    pub fn reset_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<()> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        debug!(
            "Creating Vulkan render finished semaphores for window with id: {:?}",
            self.window_id
        );

        let mut render_finished_semaphores = Vec::with_capacity(swap_chain.image_count());

        for _ in 0..swap_chain.image_count() {
            render_finished_semaphores.push(Self::create_semaphore(vulkan_context)?);
        }

        self.render_finished_semaphores = Some(render_finished_semaphores);

        Ok(())
    }

    pub fn destroy(mut self, vulkan_context: &VulkanContext) {
        let logical_device = &vulkan_context.logical_device;
        self.prepare_for_window_reset(vulkan_context);

        debug!(
            "Destroying Vulkan timeline semaphore for window with id: {:?}",
            self.window_id
        );

        unsafe {
            logical_device.destroy_semaphore(self.frame_index_semaphore, None);
        }

        debug!(
            "Destroying Vulkan image available semaphores for window with id: {:?}",
            self.window_id
        );

        for semaphore in self.image_available_semaphores.iter() {
            unsafe {
                logical_device.destroy_semaphore(*semaphore, None);
            }
        }
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

    fn create_semaphore(vulkan_context: &VulkanContext) -> NeuclidioResult<vk::Semaphore> {
        let semaphore_create_info = vk::SemaphoreCreateInfo::builder().build();

        let semaphore = unsafe {
            vulkan_context
                .logical_device
                .create_semaphore(&semaphore_create_info, None)?
        };

        Ok(semaphore)
    }
}
