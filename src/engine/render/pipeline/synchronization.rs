use crate::error::NeuclidioResult;
use vulkanalia::vk::{DeviceV1_0, Handle};
use vulkanalia::{Device, vk};

pub struct RenderPipelineSynchronization {
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub images_in_flight: Vec<vk::Fence>,
    pub max_frames_in_flight: usize,
    pub frame: usize,
}

impl RenderPipelineSynchronization {
    pub fn get_current_image_available_semaphore(&self) -> vk::Semaphore {
        self.image_available_semaphores[self.frame]
    }

    pub fn get_current_render_finished_semaphore(&self) -> vk::Semaphore {
        self.render_finished_semaphores[self.frame]
    }

    pub fn get_current_in_flight_fence(&self) -> vk::Fence {
        self.in_flight_fences[self.frame]
    }

    pub fn wait_for_in_flight_fence(&self, logical_device: &Device) -> NeuclidioResult<()> {
        let in_flight_fence = self.get_current_in_flight_fence();

        unsafe {
            logical_device.wait_for_fences(&[in_flight_fence], true, u64::MAX)?;
        }

        Ok(())
    }

    pub fn reset_current_in_flight_fence(&self, logical_device: &Device) -> NeuclidioResult<()> {
        let in_flight_fence = self.get_current_in_flight_fence();

        unsafe {
            logical_device.reset_fences(&[in_flight_fence])?;
        }

        Ok(())
    }

    pub fn wait_for_image_in_flight(
        &self,
        logical_device: &Device,
        image_index: usize,
    ) -> NeuclidioResult<()> {
        let image_in_flight = self.images_in_flight[image_index];

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
        self.images_in_flight[image_index] = in_flight_fence;

        Ok(())
    }

    pub fn increment_frame(&mut self) {
        self.frame = (self.frame + 1) % self.max_frames_in_flight;
    }
}
