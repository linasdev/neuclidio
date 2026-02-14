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
    descriptor_pool: HashMap<WindowId, vk::DescriptorPool>,
    descriptor_sets: HashMap<WindowId, Vec<vk::DescriptorSet>>,
}

impl RenderPipelineDescriptorState {
    pub fn new(
        vulkan_context: &VulkanContext,
        descriptor_set_layout_bindings: &[vk::DescriptorSetLayoutBinding],
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
        let descriptor_pool = HashMap::new();
        let descriptor_sets = HashMap::new();

        Ok(Self {
            descriptor_set_layout,
            descriptor_pool,
            descriptor_sets,
        })
    }

    pub fn prepare_for_window_reset(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) {
        let descriptor_pool = match self.descriptor_pool.remove(&neuclidio_window.id) {
            Some(descriptor_pool) => descriptor_pool,
            None => return,
        };

        debug!(
            "Destroying Vulkan descriptor pool for window with id: {:?}",
            neuclidio_window.id
        );

        unsafe {
            vulkan_context
                .logical_device
                .destroy_descriptor_pool(descriptor_pool, None);
        }
    }

    pub fn reset_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        allocator_state: &RenderPipelineAllocatorState,
        uniform_buffer_size: vk::DeviceSize,
    ) -> NeuclidioResult<()> {
        debug!(
            "Creating Vulkan descriptor pool for window with id: {:?}",
            neuclidio_window.id
        );

        let descriptor_pool = Self::create_descriptor_pool(vulkan_context, neuclidio_window)?;

        debug!(
            "Creating Vulkan descriptor sets for window with id: {:?}",
            neuclidio_window.id
        );

        let descriptor_sets = self.create_descriptor_sets(
            vulkan_context,
            neuclidio_window,
            allocator_state,
            descriptor_pool,
            uniform_buffer_size,
        )?;

        self.descriptor_pool
            .insert(neuclidio_window.id, descriptor_pool);
        self.descriptor_sets
            .insert(neuclidio_window.id, descriptor_sets);

        Ok(())
    }

    pub fn destroy(self, vulkan_context: &VulkanContext) {
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
        descriptor_set_index: usize,
    ) -> NeuclidioResult<vk::DescriptorSet> {
        self.descriptor_sets
            .get(&neuclidio_window.id)
            .map(|descriptor_sets| descriptor_sets[descriptor_set_index])
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    fn create_descriptor_pool(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<vk::DescriptorPool> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        let descriptor_pool_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(swap_chain.image_count() as u32)
            .build();

        let pool_sizes = [descriptor_pool_size];
        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(swap_chain.image_count() as u32);

        let descriptor_pool = unsafe {
            vulkan_context
                .logical_device
                .create_descriptor_pool(&descriptor_pool_create_info, None)?
        };

        Ok(descriptor_pool)
    }

    fn create_descriptor_sets(
        &self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        allocate_state: &RenderPipelineAllocatorState,
        descriptor_pool: vk::DescriptorPool,
        uniform_buffer_size: vk::DeviceSize,
    ) -> NeuclidioResult<Vec<vk::DescriptorSet>> {
        let logical_device = &vulkan_context.logical_device;
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;
        let set_layouts = vec![self.descriptor_set_layout; swap_chain.image_count()];
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts)
            .build();

        let descriptor_sets =
            unsafe { logical_device.allocate_descriptor_sets(&descriptor_set_allocate_info)? };

        for i in 0..swap_chain.image_count() {
            let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(allocate_state.uniform_buffers(neuclidio_window)?[i].0)
                .offset(0)
                .range(uniform_buffer_size)
                .build();

            let buffer_info = [descriptor_buffer_info];
            let write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_sets[i])
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
