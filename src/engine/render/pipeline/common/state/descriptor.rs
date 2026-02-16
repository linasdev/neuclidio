use crate::engine::render::pipeline::common::state::allocator::RenderPipelineAllocatorState;
use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::vulkan_context::VulkanContext;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use log::debug;
use std::collections::HashMap;
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, HasBuilder};
use winit::window::WindowId;

pub struct RenderPipelineDescriptorState {
    descriptor_set_layout: vk::DescriptorSetLayout,
    window_states: HashMap<WindowId, RenderPipelineDescriptorWindowState>,
    max_frames_in_flight: usize,
}

impl RenderPipelineDescriptorState {
    pub fn new(
        vulkan_context: &VulkanContext,
        descriptor_set_layout_bindings: &[vk::DescriptorSetLayoutBinding],
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        debug!("Creating Vulkan descriptor set layout");

        let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(descriptor_set_layout_bindings)
            .build();

        let descriptor_set_layout = unsafe {
            vulkan_context
                .logical_device
                .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)?
        };
        let window_states = HashMap::new();

        Ok(Self {
            descriptor_set_layout,
            window_states,
            max_frames_in_flight,
        })
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
        allocator_state: &RenderPipelineAllocatorState,
        uniform_buffer_size: vk::DeviceSize,
    ) -> NeuclidioResult<()> {
        if let Some(window_state) = self.window_states.get_mut(&neuclidio_window.id) {
            window_state.reset_window(
                vulkan_context,
                neuclidio_window,
                allocator_state,
                self.descriptor_set_layout,
                uniform_buffer_size,
                self.max_frames_in_flight,
            )?;
            return Ok(());
        }

        let mut window_state = RenderPipelineDescriptorWindowState::new(neuclidio_window);
        window_state.reset_window(
            vulkan_context,
            neuclidio_window,
            allocator_state,
            self.descriptor_set_layout,
            uniform_buffer_size,
            self.max_frames_in_flight,
        )?;

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

        debug!("Destroying Vulkan descriptor set layout");

        unsafe {
            vulkan_context
                .logical_device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }

    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }

    pub fn descriptor_set(
        &self,
        neuclidio_window: &NeuclidioWindow,
        frame_in_flight_index: usize,
    ) -> NeuclidioResult<vk::DescriptorSet> {
        self.window_states
            .get(&neuclidio_window.id)
            .and_then(|window_state| window_state.descriptor_sets.as_ref())
            .map(|descriptor_sets| descriptor_sets[frame_in_flight_index])
            .ok_or(RenderPipelineError::Unprepared.into())
    }
}

struct RenderPipelineDescriptorWindowState {
    window_id: WindowId,
    descriptor_pool: Option<vk::DescriptorPool>,
    descriptor_sets: Option<Vec<vk::DescriptorSet>>,
}

// TODO: Do not re-create the descriptor pool each time, reuse old one
impl RenderPipelineDescriptorWindowState {
    fn new(neuclidio_window: &NeuclidioWindow) -> Self {
        let window_id = neuclidio_window.id;

        Self {
            window_id,
            descriptor_pool: None,
            descriptor_sets: None,
        }
    }

    fn prepare_for_window_reset(&mut self, vulkan_context: &VulkanContext) {
        if let Some(descriptor_pool) = self.descriptor_pool.take() {
            debug!(
                "Destroying Vulkan descriptor pool for window with id: {:?}",
                self.window_id
            );

            unsafe {
                vulkan_context
                    .logical_device
                    .destroy_descriptor_pool(descriptor_pool, None);
            }
        }
    }

    fn reset_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        allocator_state: &RenderPipelineAllocatorState,
        descriptor_set_layout: vk::DescriptorSetLayout,
        uniform_buffer_size: vk::DeviceSize,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<()> {
        debug!(
            "Creating Vulkan descriptor pool for window with id: {:?}",
            self.window_id
        );

        let descriptor_pool = Self::create_descriptor_pool(vulkan_context, max_frames_in_flight)?;

        debug!(
            "Creating Vulkan descriptor sets for window with id: {:?}",
            self.window_id
        );

        let descriptor_sets = Self::create_descriptor_sets(
            vulkan_context,
            neuclidio_window,
            allocator_state,
            descriptor_set_layout,
            descriptor_pool,
            uniform_buffer_size,
            max_frames_in_flight,
        )?;

        self.descriptor_pool = Some(descriptor_pool);
        self.descriptor_sets = Some(descriptor_sets);

        Ok(())
    }

    fn destroy(mut self, vulkan_context: &VulkanContext) {
        self.prepare_for_window_reset(vulkan_context);
    }

    fn create_descriptor_pool(
        vulkan_context: &VulkanContext,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<vk::DescriptorPool> {
        let descriptor_pool_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(max_frames_in_flight as u32)
            .build();

        let pool_sizes = [descriptor_pool_size];
        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(max_frames_in_flight as u32);

        let descriptor_pool = unsafe {
            vulkan_context
                .logical_device
                .create_descriptor_pool(&descriptor_pool_create_info, None)?
        };

        Ok(descriptor_pool)
    }

    fn create_descriptor_sets(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        allocate_state: &RenderPipelineAllocatorState,
        descriptor_set_layout: vk::DescriptorSetLayout,
        descriptor_pool: vk::DescriptorPool,
        uniform_buffer_size: vk::DeviceSize,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Vec<vk::DescriptorSet>> {
        let logical_device = &vulkan_context.logical_device;
        let set_layouts = vec![descriptor_set_layout; max_frames_in_flight];
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts)
            .build();

        let descriptor_sets =
            unsafe { logical_device.allocate_descriptor_sets(&descriptor_set_allocate_info)? };

        for frame_in_flight_index in 0..max_frames_in_flight {
            let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(
                    allocate_state
                        .uniform_buffer(neuclidio_window, frame_in_flight_index)?
                        .0,
                )
                .offset(0)
                .range(uniform_buffer_size)
                .build();

            let buffer_info = [descriptor_buffer_info];
            let write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_sets[frame_in_flight_index])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_info)
                .build();

            unsafe {
                logical_device.update_descriptor_sets(
                    &[write_descriptor_set],
                    &[] as &[vk::CopyDescriptorSet],
                );
            }
        }

        Ok(descriptor_sets)
    }
}
