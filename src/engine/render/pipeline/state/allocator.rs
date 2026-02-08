use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::pipeline::state::command::RenderPipelineCommandState;
use crate::engine::render::pipeline::{copy_buffer, create_buffer};
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use log::debug;
use vulkanalia::vk;
use vulkanalia_vma::{Allocation, AllocationCreateFlags, Allocator, AllocatorOptions, MemoryUsage};

pub struct RenderPipelineAllocatorState {
    allocator: Allocator,
    render_buffer: Option<(vk::Buffer, Allocation)>,
    uniform_buffers: Option<Vec<(vk::Buffer, Allocation)>>,
}

impl RenderPipelineAllocatorState {
    pub fn new(neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<Self> {
        debug!(
            "Creating Vulkan Memory Allocator for window with id: {:?}",
            neuclidio_window.id
        );

        let allocator_options = AllocatorOptions::new(
            &neuclidio_window.instance,
            &neuclidio_window.logical_device,
            neuclidio_window.physical_device,
        );
        let allocator = unsafe { Allocator::new(&allocator_options)? };

        Ok(Self {
            allocator,
            render_buffer: None,
            uniform_buffers: None,
        })
    }

    pub fn prepare_for_reset(&mut self, neuclidio_window: &NeuclidioWindow) {
        let uniform_buffers = match self.uniform_buffers.take() {
            Some(uniform_buffers) => uniform_buffers,
            None => return,
        };

        debug!(
            "Destroying Vulkan uniform buffers for window with id: {:?}",
            neuclidio_window.id
        );

        for uniform_buffer in uniform_buffers.iter() {
            unsafe {
                self.allocator
                    .destroy_buffer(uniform_buffer.0, uniform_buffer.1);
            }
        }
    }

    pub fn reset(
        &mut self,
        neuclidio_window: &NeuclidioWindow,
        uniform_buffer_size: vk::DeviceSize,
    ) -> NeuclidioResult<()> {
        debug!(
            "Creating Vulkan uniform buffers for window with id: {:?}",
            neuclidio_window.id
        );

        let uniform_buffers =
            Self::create_uniform_buffers(neuclidio_window, &self.allocator, uniform_buffer_size)?;

        self.uniform_buffers = Some(uniform_buffers);

        Ok(())
    }

    pub fn destroy(mut self, neuclidio_window: &NeuclidioWindow) {
        if let Some(render_buffer) = self.render_buffer.take() {
            debug!(
                "Destroying Vulkan render buffer for window with id: {:?}",
                neuclidio_window.id
            );

            unsafe {
                self.allocator
                    .destroy_buffer(render_buffer.0, render_buffer.1);
            }
        }

        if let Some(uniform_buffers) = self.uniform_buffers.take() {
            debug!(
                "Destroying Vulkan uniform buffers for window with id: {:?}",
                neuclidio_window.id
            );

            for uniform_buffer in uniform_buffers.iter() {
                unsafe {
                    self.allocator
                        .destroy_buffer(uniform_buffer.0, uniform_buffer.1);
                }
            }
        }

        debug!(
            "Destroying Vulkan Memory Allocator for window with id: {:?}",
            neuclidio_window.id
        );

        drop(self.allocator);
    }

    pub fn fill_render_buffer<RBF>(
        &mut self,
        neuclidio_window: &NeuclidioWindow,
        command_state: &RenderPipelineCommandState,
        render_buffer_size: vk::DeviceSize,
        mut render_buffer_filler: RBF,
    ) -> NeuclidioResult<()>
    where
        RBF: FnMut(*mut u8),
    {
        if let Some(render_buffer) = self.render_buffer.take() {
            debug!(
                "Destroying Vulkan render buffer for window with id: {:?}",
                neuclidio_window.id
            );

            unsafe {
                self.allocator
                    .destroy_buffer(render_buffer.0, render_buffer.1);
            }
        }

        debug!(
            "Creating Vulkan render buffer for window with id: {:?}",
            neuclidio_window.id
        );

        let staging_render_buffer = create_buffer(
            &self.allocator,
            render_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryUsage::AutoPreferHost,
            AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
        )?;

        let render_buffer_memory: *mut u8 =
            unsafe { self.allocator.map_memory(staging_render_buffer.1)? };

        render_buffer_filler(render_buffer_memory);

        unsafe {
            self.allocator.unmap_memory(staging_render_buffer.1);
            self.allocator
                .flush_allocation(staging_render_buffer.1, 0, vk::WHOLE_SIZE)?;
        }

        let render_buffer = create_buffer(
            &self.allocator,
            render_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::INDEX_BUFFER,
            MemoryUsage::AutoPreferDevice,
            AllocationCreateFlags::empty(),
        )?;

        copy_buffer(
            neuclidio_window,
            command_state.command_pool(),
            render_buffer_size,
            staging_render_buffer.0,
            render_buffer.0,
        )?;

        unsafe {
            self.allocator
                .destroy_buffer(staging_render_buffer.0, staging_render_buffer.1);
        }

        self.render_buffer = Some(render_buffer);

        Ok(())
    }

    pub fn fill_uniform_buffer<UBF>(
        &mut self,
        image_index: usize,
        mut uniform_buffer_filler: UBF,
    ) -> NeuclidioResult<()>
    where
        UBF: FnMut(*mut u8) -> NeuclidioResult<()>,
    {
        let uniform_buffer = self.uniform_buffers()?[image_index];

        let uniform_buffer_memory: *mut u8 =
            unsafe { self.allocator.map_memory(uniform_buffer.1)? };

        uniform_buffer_filler(uniform_buffer_memory)?;

        unsafe {
            self.allocator.unmap_memory(uniform_buffer.1);
            self.allocator
                .flush_allocation(uniform_buffer.1, 0, vk::WHOLE_SIZE)?;
        }

        Ok(())
    }

    pub fn render_buffer(&self) -> Option<vk::Buffer> {
        self.render_buffer.map(|render_buffer| render_buffer.0)
    }

    pub fn uniform_buffers(&self) -> NeuclidioResult<&[(vk::Buffer, Allocation)]> {
        self.uniform_buffers
            .as_ref()
            .map(|uniform_buffers| &uniform_buffers[..])
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    fn create_uniform_buffers(
        neuclidio_window: &NeuclidioWindow,
        allocator: &Allocator,
        uniform_buffer_size: vk::DeviceSize,
    ) -> NeuclidioResult<Vec<(vk::Buffer, Allocation)>> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;
        let mut uniform_buffers = Vec::with_capacity(swap_chain.image_count());

        for _ in 0..swap_chain.image_count() {
            let uniform_buffer = create_buffer(
                allocator,
                uniform_buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                MemoryUsage::AutoPreferHost,
                AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            )?;

            uniform_buffers.push(uniform_buffer);
        }

        Ok(uniform_buffers)
    }
}
