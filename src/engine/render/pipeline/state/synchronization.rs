use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use log::debug;
use vulkanalia::vk::{DeviceV1_0, Handle, HasBuilder};
use vulkanalia::{Device, vk};

pub struct RenderPipelineSynchronizationState {
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    images_in_flight: Option<Vec<vk::Fence>>,
    max_frames_in_flight: usize,
    frame: usize,
}

impl RenderPipelineSynchronizationState {
    pub fn new(
        neuclidio_window: &NeuclidioWindow,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        debug!(
            "Creating Vulkan synchronization objects for window with id: {:?}",
            neuclidio_window.id
        );

        let mut image_available_semaphores = Vec::with_capacity(max_frames_in_flight);
        let mut render_finished_semaphores = Vec::with_capacity(max_frames_in_flight);
        let mut in_flight_fences = Vec::with_capacity(max_frames_in_flight);

        for _ in 0..max_frames_in_flight {
            image_available_semaphores.push(Self::create_semaphore(neuclidio_window)?);
            render_finished_semaphores.push(Self::create_semaphore(neuclidio_window)?);
            in_flight_fences.push(Self::create_fence(neuclidio_window)?);
        }

        let synchronization = Self {
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight: None,
            max_frames_in_flight,
            frame: 0,
        };

        Ok(synchronization)
    }

    pub fn prepare_for_reset(&mut self) {
        self.images_in_flight.take();
    }

    pub fn reset(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        self.images_in_flight = Some(vec![vk::Fence::null(); swap_chain.image_count()]);

        Ok(())
    }

    pub fn destroy(self, neuclidio_window: &NeuclidioWindow) {
        let logical_device = &neuclidio_window.logical_device;

        debug!(
            "Destroying Vulkan fences for window with id: {:?}",
            neuclidio_window.id
        );

        for fence in self.in_flight_fences.iter() {
            unsafe {
                logical_device.destroy_fence(*fence, None);
            }
        }

        debug!(
            "Destroying Vulkan semaphores for window with id: {:?}",
            neuclidio_window.id
        );

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

    pub fn get_current_image_available_semaphore(&self) -> vk::Semaphore {
        self.image_available_semaphores[self.frame]
    }

    pub fn get_current_render_finished_semaphore(&self) -> vk::Semaphore {
        self.render_finished_semaphores[self.frame]
    }

    pub fn get_current_in_flight_fence(&self) -> vk::Fence {
        self.in_flight_fences[self.frame]
    }

    pub fn wait_for_in_flight_fence(
        &self,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<()> {
        let in_flight_fence = self.get_current_in_flight_fence();

        unsafe {
            neuclidio_window
                .logical_device
                .wait_for_fences(&[in_flight_fence], true, u64::MAX)?;
        }

        Ok(())
    }

    pub fn reset_current_in_flight_fence(
        &self,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<()> {
        let in_flight_fence = self.get_current_in_flight_fence();

        unsafe {
            neuclidio_window
                .logical_device
                .reset_fences(&[in_flight_fence])?;
        }

        Ok(())
    }

    pub fn wait_for_image_in_flight(
        &self,
        logical_device: &Device,
        image_index: usize,
    ) -> NeuclidioResult<()> {
        let images_in_flight = match self.images_in_flight.as_ref() {
            Some(image_in_flight) => image_in_flight,
            None => return Err(RenderPipelineError::Unprepared.into()),
        };

        let image_in_flight = images_in_flight[image_index];

        if image_in_flight.is_null() {
            return Ok(());
        }

        unsafe {
            logical_device.wait_for_fences(&[image_in_flight], true, u64::MAX)?;
        }

        Ok(())
    }

    pub fn set_image_in_flight_to_current_in_flight_fence(
        &mut self,
        image_index: usize,
    ) -> NeuclidioResult<()> {
        let in_flight_fence = self.get_current_in_flight_fence();

        let images_in_flight = match self.images_in_flight.as_mut() {
            Some(image_in_flight) => image_in_flight,
            None => return Err(RenderPipelineError::Unprepared.into()),
        };
        images_in_flight[image_index] = in_flight_fence;

        Ok(())
    }

    pub fn increment_frame(&mut self) {
        self.frame = (self.frame + 1) % self.max_frames_in_flight;
    }

    fn create_semaphore(neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<vk::Semaphore> {
        let semaphore_create_info = vk::SemaphoreCreateInfo::builder().build();

        let semaphore = unsafe {
            neuclidio_window
                .logical_device
                .create_semaphore(&semaphore_create_info, None)?
        };

        Ok(semaphore)
    }

    fn create_fence(neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<vk::Fence> {
        let fence_create_info = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED)
            .build();

        let fence = unsafe {
            neuclidio_window
                .logical_device
                .create_fence(&fence_create_info, None)?
        };

        Ok(fence)
    }
}
