use crate::engine::render::pipeline::common::state::synchronization::RenderPipelineSynchronizationState;
use crate::engine::render::pipeline::common::state::transfer::RenderPipelineTransferState;
use crate::engine::render::pipeline::create_buffer;
use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::renderable::{Renderable, RenderableExt, RenderableMemoryAllocation};
use crate::engine::render::vulkan_context::VulkanContext;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::{NeuclidioError, NeuclidioResult};
use crate::id::RenderBufferId;
use log::{debug, trace, warn};
use std::cmp::max;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, DeviceV1_3, Handle, HasBuilder};
use vulkanalia_vma::{
    Alloc, Allocation, AllocationCreateFlags, AllocationOptions, Allocator, MemoryUsage,
    VirtualAllocationCreateFlags, VirtualAllocationOptions, VirtualBlock, VirtualBlockCreateFlags,
    VirtualBlockOptions,
};
use winit::window::WindowId;

const RENDER_BUFFER_ALIGNMENT: vk::DeviceSize = 16;
const MIN_RENDER_BUFFER_SIZE: vk::DeviceSize = 1024 * 1024 * 64;

pub struct RenderBuffer {
    buffer: vk::Buffer,
    allocation: Allocation,
    virtual_block: VirtualBlock,
    renderable_count: usize,
}

pub struct RenderPipelineAllocatorState {
    render_buffers: HashMap<RenderBufferId, RenderBuffer>,
    renderables_pending_deallocation: Vec<Renderable>,
    color_image_format: vk::Format,
    depth_stencil_image_format: vk::Format,
    window_states: HashMap<WindowId, RenderPipelineAllocatorWindowState>,
    max_frames_in_flight: usize,
}

impl RenderPipelineAllocatorState {
    pub fn new(
        color_image_format: vk::Format,
        depth_stencil_image_format: vk::Format,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        Ok(Self {
            render_buffers: HashMap::new(),
            renderables_pending_deallocation: vec![],
            color_image_format,
            depth_stencil_image_format,
            window_states: HashMap::new(),
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
        uniform_buffer_size: vk::DeviceSize,
    ) -> NeuclidioResult<()> {
        if let Some(window_state) = self.window_states.get_mut(&neuclidio_window.id) {
            window_state.reset_window(
                vulkan_context,
                neuclidio_window,
                self.color_image_format,
                self.depth_stencil_image_format,
                self.max_frames_in_flight,
            )?;
            return Ok(());
        }

        let mut window_state = RenderPipelineAllocatorWindowState::new(
            vulkan_context,
            neuclidio_window,
            uniform_buffer_size,
            self.max_frames_in_flight,
        )?;
        window_state.reset_window(
            vulkan_context,
            neuclidio_window,
            self.color_image_format,
            self.depth_stencil_image_format,
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

        debug!("Destroying Vulkan render buffers");

        for render_buffer in self.render_buffers.into_values() {
            render_buffer.virtual_block.clear();
            unsafe {
                vulkan_context
                    .allocator
                    .destroy_buffer(render_buffer.buffer, render_buffer.allocation);
            }
        }
    }

    pub fn submit_renderables(
        &mut self,
        vulkan_context: &VulkanContext,
        transfer_state: &RenderPipelineTransferState,
        renderables: &[Renderable],
    ) -> NeuclidioResult<()> {
        for renderable in renderables {
            self.allocate_renderable(vulkan_context, renderable)?;
        }

        let mut uncopied_renderables: Vec<&Renderable> = vec![];
        for renderable in renderables {
            if let Some(uncopied_renderable) = uncopied_renderables.last() {
                let mut should_copy = false;

                if renderable.render_buffer_id() != uncopied_renderable.render_buffer_id() {
                    should_copy = true;
                }

                let end_offset_of_uncoped_renderable = uncopied_renderable
                    .render_buffer_offset()
                    .map(|offset| offset + uncopied_renderable.size_in_render_buffer());
                if renderable.render_buffer_offset() != end_offset_of_uncoped_renderable {
                    should_copy = true;
                }

                if !should_copy {
                    uncopied_renderables.push(renderable);
                    continue;
                }

                self.copy_to_render_buffer(vulkan_context, transfer_state, uncopied_renderables)?;
                uncopied_renderables = vec![];

                continue;
            }

            uncopied_renderables.push(renderable);
        }

        if !uncopied_renderables.is_empty() {
            self.copy_to_render_buffer(vulkan_context, transfer_state, uncopied_renderables)?;
        }

        Ok(())
    }

    pub fn remove_renderables(&mut self, renderables: &[Renderable]) -> NeuclidioResult<()> {
        for renderable in renderables {
            self.renderables_pending_deallocation
                .push(renderable.clone());
        }

        Ok(())
    }

    pub fn deallocate_renderables(
        &mut self,
        vulkan_context: &VulkanContext,
        synchronization_state: &RenderPipelineSynchronizationState,
    ) -> NeuclidioResult<()> {
        let frame_index_semaphore_values = self
            .window_states
            .keys()
            .map(|window_id| {
                Ok((
                    *window_id,
                    synchronization_state
                        .frame_index_semaphore_value_by_window_id(vulkan_context, *window_id)?,
                ))
            })
            .collect::<NeuclidioResult<HashMap<_, _>>>()?;

        let mut renderables_to_deallocate = vec![];
        for (renderable_index, renderable) in
            self.renderables_pending_deallocation.iter().enumerate()
        {
            if let Some(last_used_in_frame) = renderable.last_used_in_frame() {
                let mut still_used = false;
                let last_used_in_frame = last_used_in_frame.lock().unwrap();
                for (window_id, last_used_in_frame) in last_used_in_frame.iter() {
                    if let Some(frame_index_semaphore_value) =
                        frame_index_semaphore_values.get(window_id)
                        && last_used_in_frame > frame_index_semaphore_value
                    {
                        still_used = true;
                        break;
                    }
                }

                if still_used {
                    continue;
                }

                renderables_to_deallocate.push(renderable_index);

                continue;
            }

            warn!("Tried to deallocate a renderable that was never used in the render pipeline");
        }

        for renderable_index in renderables_to_deallocate.into_iter().rev() {
            let renderable = self
                .renderables_pending_deallocation
                .remove(renderable_index);
            self.deallocate_renderable(vulkan_context, &renderable)?;
        }

        Ok(())
    }

    pub fn fill_uniform_buffer<UBF>(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        frame_in_flight_index: usize,
        mut uniform_buffer_filler: UBF,
    ) -> NeuclidioResult<()>
    where
        UBF: FnMut(*mut u8) -> NeuclidioResult<()>,
    {
        let uniform_buffer = self.uniform_buffer(neuclidio_window, frame_in_flight_index)?;

        let uniform_buffer_memory: *mut u8 =
            unsafe { vulkan_context.allocator.map_memory(uniform_buffer.1)? };

        uniform_buffer_filler(uniform_buffer_memory)?;

        unsafe {
            vulkan_context.allocator.unmap_memory(uniform_buffer.1);
            vulkan_context
                .allocator
                .flush_allocation(uniform_buffer.1, 0, vk::WHOLE_SIZE)?;
        }

        Ok(())
    }

    pub fn render_buffer(&self, render_buffer_id: RenderBufferId) -> Option<vk::Buffer> {
        self.render_buffers
            .get(&render_buffer_id)
            .map(|render_buffer| render_buffer.buffer)
    }

    pub fn color_image_format(&self) -> vk::Format {
        self.color_image_format
    }

    pub fn color_image_view(
        &self,
        neuclidio_window: &NeuclidioWindow,
        frame_in_flight_index: usize,
    ) -> NeuclidioResult<vk::ImageView> {
        self.window_states
            .get(&neuclidio_window.id)
            .and_then(|window_state| window_state.color_image_views.as_ref())
            .map(|color_image_views| color_image_views[frame_in_flight_index])
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    pub fn depth_stencil_image_format(&self) -> vk::Format {
        self.depth_stencil_image_format
    }

    pub fn depth_stencil_image_view(
        &self,
        neuclidio_window: &NeuclidioWindow,
        frame_in_flight_index: usize,
    ) -> NeuclidioResult<vk::ImageView> {
        self.window_states
            .get(&neuclidio_window.id)
            .and_then(|window_state| window_state.depth_stencil_image_views.as_ref())
            .map(|depth_stencil_image_views| depth_stencil_image_views[frame_in_flight_index])
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    pub fn uniform_buffer(
        &self,
        neuclidio_window: &NeuclidioWindow,
        frame_in_flight_index: usize,
    ) -> NeuclidioResult<&(vk::Buffer, Allocation)> {
        self.window_states
            .get(&neuclidio_window.id)
            .map(|window_state| &window_state.uniform_buffers[frame_in_flight_index])
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    fn allocate_renderable(
        &mut self,
        vulkan_context: &VulkanContext,
        renderable: &Renderable,
    ) -> NeuclidioResult<()> {
        if renderable.render_buffer_id().is_some() {
            warn!(
                "Tried to double allocate a renderable with id: {:?}",
                renderable.id()
            );
            return Ok(());
        }

        let virtual_allocation_size = renderable.size_in_render_buffer();
        let virtual_allocation_options = VirtualAllocationOptions {
            size: virtual_allocation_size,
            alignment: RENDER_BUFFER_ALIGNMENT,
            flags: VirtualAllocationCreateFlags::empty(),
        };

        for (&render_buffer_id, render_buffer) in self.render_buffers.iter_mut() {
            if let Ok((virtual_allocation, offset)) = render_buffer
                .virtual_block
                .allocate(&virtual_allocation_options)
            {
                let renderable_memory_allocation = RenderableMemoryAllocation {
                    render_buffer_id,
                    virtual_allocation,
                    offset,
                    last_used_in_frame: Arc::new(Mutex::new(HashMap::new())),
                };

                renderable.set_memory_allocation(Some(renderable_memory_allocation));
                render_buffer.renderable_count += 1;

                return Ok(());
            }
        }

        let render_buffer_id = RenderBufferId::new();
        debug!("Creating Vulkan render buffer with id: {render_buffer_id}");

        let virtual_block_size = max(virtual_allocation_size, MIN_RENDER_BUFFER_SIZE);
        let virtual_block_options = VirtualBlockOptions {
            size: virtual_block_size,
            flags: VirtualBlockCreateFlags::empty(),
        };

        let (buffer, allocation) = create_buffer(
            &vulkan_context.allocator,
            virtual_block_size,
            vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::INDEX_BUFFER,
            MemoryUsage::AutoPreferDevice,
            AllocationCreateFlags::empty(),
        )?;
        let virtual_block = VirtualBlock::new(&virtual_block_options)?;
        let render_buffer = RenderBuffer {
            buffer,
            allocation,
            virtual_block,
            renderable_count: 1,
        };

        let (virtual_allocation, offset) = render_buffer
            .virtual_block
            .allocate(&virtual_allocation_options)?;
        let renderable_memory_allocation = RenderableMemoryAllocation {
            render_buffer_id,
            virtual_allocation,
            offset,
            last_used_in_frame: Arc::new(Mutex::new(HashMap::new())),
        };

        renderable.set_memory_allocation(Some(renderable_memory_allocation));

        self.render_buffers.insert(render_buffer_id, render_buffer);

        Ok(())
    }

    fn deallocate_renderable(
        &mut self,
        vulkan_context: &VulkanContext,
        renderable: &Renderable,
    ) -> NeuclidioResult<()> {
        if let Some(renderable_memory_allocation) = renderable.set_memory_allocation(None)
            && let Some(render_buffer) = self
                .render_buffers
                .get_mut(&renderable_memory_allocation.render_buffer_id)
        {
            render_buffer
                .virtual_block
                .free(renderable_memory_allocation.virtual_allocation);

            render_buffer.renderable_count -= 1;

            let render_buffer_id = renderable_memory_allocation.render_buffer_id;
            if render_buffer.renderable_count == 0 {
                if let Some(render_buffer) = self.render_buffers.remove(&render_buffer_id) {
                    debug!("Destroying Vulkan render buffer with id: {render_buffer_id}");

                    unsafe {
                        vulkan_context
                            .allocator
                            .destroy_buffer(render_buffer.buffer, render_buffer.allocation);
                    }
                }
            }

            return Ok(());
        }

        warn!(
            "Tried to double deallocate a renderable with id: {:?}",
            renderable.id()
        );
        Ok(())
    }

    fn copy_to_render_buffer(
        &self,
        vulkan_context: &VulkanContext,
        transfer_state: &RenderPipelineTransferState,
        renderables: Vec<&Renderable>,
    ) -> NeuclidioResult<()> {
        trace!(
            "Creating Vulkan staging render buffer and copying {} renderables with it",
            renderables.len()
        );

        let first_renderable = renderables
            .first()
            .ok_or::<NeuclidioError>(RenderPipelineError::RenderableNotAllocated.into())?;

        let render_buffer = first_renderable
            .render_buffer_id()
            .and_then(|render_buffer_id| self.render_buffers.get(&render_buffer_id))
            .map(|render_buffer| render_buffer.buffer)
            .ok_or::<NeuclidioError>(RenderPipelineError::RenderableNotAllocated.into())?;

        let render_buffer_offset = first_renderable
            .render_buffer_offset()
            .ok_or::<NeuclidioError>(RenderPipelineError::RenderableNotAllocated.into())?;

        let staging_render_buffer_size = renderables
            .iter()
            .map(|renderable| renderable.size_in_render_buffer())
            .sum();

        let staging_render_buffer = create_buffer(
            &vulkan_context.allocator,
            staging_render_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryUsage::AutoPreferHost,
            AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
        )?;

        let staging_render_buffer_memory: *mut u8 = unsafe {
            vulkan_context
                .allocator
                .map_memory(staging_render_buffer.1)?
        };

        for renderable in renderables {
            renderable.load_into_staging_render_buffer(staging_render_buffer_memory);
        }

        unsafe {
            vulkan_context
                .allocator
                .unmap_memory(staging_render_buffer.1);
            vulkan_context.allocator.flush_allocation(
                staging_render_buffer.1,
                0,
                vk::WHOLE_SIZE,
            )?;
        }

        Self::transfer_render_buffer_slice(
            vulkan_context,
            transfer_state.command_pool(),
            staging_render_buffer_size,
            staging_render_buffer.0,
            render_buffer,
            0,
            render_buffer_offset,
        )?;

        unsafe {
            vulkan_context
                .allocator
                .destroy_buffer(staging_render_buffer.0, staging_render_buffer.1);
        }

        Ok(())
    }

    fn transfer_render_buffer_slice(
        vulkan_context: &VulkanContext,
        command_pool: vk::CommandPool,
        size: vk::DeviceSize,
        source: vk::Buffer,
        destination: vk::Buffer,
        source_offset: vk::DeviceSize,
        destination_offset: vk::DeviceSize,
    ) -> NeuclidioResult<()> {
        let logical_device = &vulkan_context.logical_device;
        let transfer_queue = vulkan_context.transfer_queue;

        let buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(1)
            .build();

        let command_buffer =
            unsafe { logical_device.allocate_command_buffers(&buffer_allocate_info)?[0] };

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();

        unsafe {
            logical_device.begin_command_buffer(command_buffer, &command_buffer_begin_info)?;
        }

        let buffer_copy = vk::BufferCopy::builder()
            .src_offset(source_offset)
            .dst_offset(destination_offset)
            .size(size)
            .build();

        unsafe {
            logical_device.cmd_copy_buffer(command_buffer, source, destination, &[buffer_copy]);

            if vulkan_context.queue_family_indices.transfer
                != vulkan_context.queue_family_indices.graphics
            {
                let buffer_memory_barrier = vk::BufferMemoryBarrier2::builder()
                    .src_stage_mask(vk::PipelineStageFlags2::COPY)
                    .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                    .src_queue_family_index(vulkan_context.queue_family_indices.transfer)
                    .dst_stage_mask(
                        vk::PipelineStageFlags2::INDEX_INPUT
                            | vk::PipelineStageFlags2::VERTEX_ATTRIBUTE_INPUT,
                    )
                    .dst_access_mask(
                        vk::AccessFlags2::INDEX_READ | vk::AccessFlags2::VERTEX_ATTRIBUTE_READ,
                    )
                    .dst_queue_family_index(vulkan_context.queue_family_indices.graphics)
                    .buffer(destination)
                    .offset(destination_offset)
                    .size(size)
                    .build();

                let buffer_memory_barriers = [buffer_memory_barrier];
                let dependency_info = vk::DependencyInfo::builder()
                    .buffer_memory_barriers(&buffer_memory_barriers)
                    .build();

                logical_device.cmd_pipeline_barrier2(command_buffer, &dependency_info)
            }

            logical_device.end_command_buffer(command_buffer)?;
        }

        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .build();

        unsafe {
            logical_device.queue_submit(transfer_queue, &[submit_info], vk::Fence::null())?;
            logical_device.queue_wait_idle(transfer_queue)?; // TODO: Replace this idle waiting with a command buffer deletion queue
            logical_device.free_command_buffers(command_pool, &command_buffers);
        }

        Ok(())
    }
}

struct RenderPipelineAllocatorWindowState {
    window_id: WindowId,
    uniform_buffers: Vec<(vk::Buffer, Allocation)>,
    color_images: Option<Vec<(vk::Image, Allocation)>>,
    color_image_views: Option<Vec<vk::ImageView>>,
    depth_stencil_images: Option<Vec<(vk::Image, Allocation)>>,
    depth_stencil_image_views: Option<Vec<vk::ImageView>>,
}

impl RenderPipelineAllocatorWindowState {
    fn new(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        uniform_buffer_size: vk::DeviceSize,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        let window_id = neuclidio_window.id;

        debug!("Creating Vulkan uniform buffers for window with id: {window_id:?}");

        let uniform_buffers = Self::create_uniform_buffers(
            vulkan_context,
            uniform_buffer_size,
            max_frames_in_flight,
        )?;

        Ok(Self {
            window_id,
            uniform_buffers,
            color_images: None,
            color_image_views: None,
            depth_stencil_images: None,
            depth_stencil_image_views: None,
        })
    }

    fn prepare_for_window_reset(&mut self, vulkan_context: &VulkanContext) {
        debug!(
            "Destroying Vulkan depth / stencil image views for window with id: {:?}",
            self.window_id
        );

        if let Some(depth_stencil_image_views) = self.depth_stencil_image_views.take() {
            for depth_stencil_image_view in depth_stencil_image_views.into_iter() {
                unsafe {
                    vulkan_context
                        .logical_device
                        .destroy_image_view(depth_stencil_image_view, None);
                }
            }
        }

        debug!(
            "Destroying Vulkan depth / stencil images for window with id: {:?}",
            self.window_id
        );

        if let Some(depth_stencil_images) = self.depth_stencil_images.take() {
            for depth_stencil_image in depth_stencil_images.into_iter() {
                unsafe {
                    vulkan_context
                        .allocator
                        .destroy_image(depth_stencil_image.0, depth_stencil_image.1);
                }
            }
        }

        debug!(
            "Destroying Vulkan color image views for window with id: {:?}",
            self.window_id
        );

        if let Some(color_image_views) = self.color_image_views.take() {
            for color_image_view in color_image_views.into_iter() {
                unsafe {
                    vulkan_context
                        .logical_device
                        .destroy_image_view(color_image_view, None);
                }
            }
        }

        debug!(
            "Destroying Vulkan color images for window with id: {:?}",
            self.window_id
        );

        if let Some(color_images) = self.color_images.take() {
            for color_image in color_images.into_iter() {
                unsafe {
                    vulkan_context
                        .allocator
                        .destroy_image(color_image.0, color_image.1);
                }
            }
        }
    }

    fn reset_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        color_image_format: vk::Format,
        depth_stencil_image_format: vk::Format,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<()> {
        debug!(
            "Creating Vulkan color images for window with id: {:?}",
            self.window_id
        );

        let color_images = Self::create_images(
            neuclidio_window,
            &vulkan_context.allocator,
            color_image_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            max_frames_in_flight,
        )?;

        debug!(
            "Creating Vulkan color image views for window with id: {:?}",
            self.window_id
        );

        let color_image_views = Self::create_image_views(
            vulkan_context,
            color_image_format,
            vk::ImageAspectFlags::COLOR,
            &color_images,
        )?;

        debug!(
            "Creating Vulkan depth / stencil images for window with id: {:?}",
            self.window_id
        );

        let depth_stencil_images = Self::create_images(
            neuclidio_window,
            &vulkan_context.allocator,
            depth_stencil_image_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            max_frames_in_flight,
        )?;

        debug!(
            "Creating Vulkan depth / stencil image views for window with id: {:?}",
            self.window_id
        );

        let depth_stencil_image_views = Self::create_image_views(
            vulkan_context,
            depth_stencil_image_format,
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
            &depth_stencil_images,
        )?;

        self.color_images = Some(color_images);
        self.color_image_views = Some(color_image_views);
        self.depth_stencil_images = Some(depth_stencil_images);
        self.depth_stencil_image_views = Some(depth_stencil_image_views);

        Ok(())
    }

    fn destroy(mut self, vulkan_context: &VulkanContext) {
        self.prepare_for_window_reset(vulkan_context);

        debug!(
            "Destroying Vulkan uniform buffers for window with id: {:?}",
            self.window_id
        );

        for uniform_buffer in self.uniform_buffers.into_iter() {
            unsafe {
                vulkan_context
                    .allocator
                    .destroy_buffer(uniform_buffer.0, uniform_buffer.1);
            }
        }
    }

    fn create_images(
        neuclidio_window: &NeuclidioWindow,
        allocator: &Allocator,
        image_format: vk::Format,
        image_tiling: vk::ImageTiling,
        image_usage: vk::ImageUsageFlags,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Vec<(vk::Image, Allocation)>> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        let image_extent = vk::Extent3D::builder()
            .width(swap_chain.extent().width)
            .height(swap_chain.extent().height)
            .depth(1)
            .build();

        let mut images = Vec::with_capacity(max_frames_in_flight);

        for _ in 0..max_frames_in_flight {
            let image_create_info = vk::ImageCreateInfo::builder()
                .flags(vk::ImageCreateFlags::empty())
                .image_type(vk::ImageType::_2D)
                .format(image_format)
                .extent(image_extent)
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::_1)
                .tiling(image_tiling)
                .usage(image_usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .build();

            let mut allocation_options = AllocationOptions::default();
            allocation_options.usage = MemoryUsage::AutoPreferDevice;

            let image = unsafe { allocator.create_image(image_create_info, &allocation_options)? };

            images.push(image);
        }

        Ok(images)
    }

    fn create_image_views(
        vulkan_context: &VulkanContext,
        image_format: vk::Format,
        image_aspect: vk::ImageAspectFlags,
        images: &[(vk::Image, Allocation)],
    ) -> NeuclidioResult<Vec<vk::ImageView>> {
        let mut image_views = Vec::with_capacity(images.len());

        for (image, _) in images.iter() {
            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(image_aspect)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            let image_view_create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::_2D)
                .format(image_format)
                .subresource_range(subresource_range);

            let image_view = unsafe {
                vulkan_context
                    .logical_device
                    .create_image_view(&image_view_create_info, None)?
            };

            image_views.push(image_view);
        }

        Ok(image_views)
    }

    fn create_uniform_buffers(
        vulkan_context: &VulkanContext,
        uniform_buffer_size: vk::DeviceSize,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Vec<(vk::Buffer, Allocation)>> {
        let mut uniform_buffers = Vec::with_capacity(max_frames_in_flight);

        for _ in 0..max_frames_in_flight {
            let uniform_buffer = create_buffer(
                &vulkan_context.allocator,
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
