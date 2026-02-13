use crate::engine::render::error::RenderError;
use crate::engine::render::pipeline::common::state::command::RenderPipelineCommandState;
use crate::engine::render::pipeline::common::state::synchronization::RenderPipelineSynchronizationState;
use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::pipeline::{copy_buffer, create_buffer};
use crate::engine::render::renderable::{Renderable, RenderableExt, RenderableMemoryAllocation};
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::{NeuclidioError, NeuclidioResult};
use crate::id_generator::IdGenerator;
use log::{debug, trace, warn};
use std::cmp::max;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, HasBuilder, InstanceV1_0};
use vulkanalia_vma::{
    Alloc, Allocation, AllocationCreateFlags, AllocationOptions, Allocator, AllocatorOptions,
    MemoryUsage, VirtualAllocationCreateFlags, VirtualAllocationOptions, VirtualBlock,
    VirtualBlockCreateFlags, VirtualBlockOptions,
};

const RENDER_BUFFER_ALIGNMENT: vk::DeviceSize = 16;
const MIN_RENDER_BUFFER_SIZE: vk::DeviceSize = 1024 * 1024 * 64;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct RenderBufferId(pub(crate) u64);

pub struct RenderBuffer {
    buffer: vk::Buffer,
    allocation: Allocation,
    virtual_block: VirtualBlock,
    renderable_count: usize,
}

pub struct RenderPipelineAllocatorState {
    allocator: Allocator,
    render_buffers: HashMap<RenderBufferId, RenderBuffer>,
    renderables_pending_deallocation: Vec<Renderable>,
    depth_stencil_image_format: Option<vk::Format>,
    depth_stencil_image: Option<(vk::Image, Allocation)>,
    depth_stencil_image_view: Option<vk::ImageView>,
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
            render_buffers: HashMap::new(),
            renderables_pending_deallocation: vec![],
            depth_stencil_image_format: None,
            depth_stencil_image: None,
            depth_stencil_image_view: None,
            uniform_buffers: None,
        })
    }

    pub fn prepare_for_reset(&mut self, neuclidio_window: &NeuclidioWindow) {
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

        if let Some(depth_stencil_image_view) = self.depth_stencil_image_view.take() {
            debug!(
                "Destroying Vulkan depth image view for window with id: {:?}",
                neuclidio_window.id
            );

            unsafe {
                neuclidio_window
                    .logical_device
                    .destroy_image_view(depth_stencil_image_view, None);
            }
        }

        if let Some(depth_stencil_image) = self.depth_stencil_image.take() {
            debug!(
                "Destroying Vulkan depth image for window with id: {:?}",
                neuclidio_window.id
            );

            unsafe {
                self.allocator
                    .destroy_image(depth_stencil_image.0, depth_stencil_image.1);
            }
        }
    }

    pub fn reset(
        &mut self,
        neuclidio_window: &NeuclidioWindow,
        uniform_buffer_size: vk::DeviceSize,
    ) -> NeuclidioResult<()> {
        debug!(
            "Creating Vulkan depth image for window with id: {:?}",
            neuclidio_window.id
        );

        let depth_stencil_image_format = Self::get_image_format(
            neuclidio_window,
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
            &[
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D24_UNORM_S8_UINT,
            ],
        )?;
        let depth_stencil_image = Self::create_depth_stencil_image(
            neuclidio_window,
            &self.allocator,
            depth_stencil_image_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        )?;
        let depth_stencil_image_view = Self::create_depth_stencil_image_view(
            neuclidio_window,
            depth_stencil_image.0,
            depth_stencil_image_format,
        )?;

        debug!(
            "Creating Vulkan uniform buffers for window with id: {:?}",
            neuclidio_window.id
        );

        let uniform_buffers =
            Self::create_uniform_buffers(neuclidio_window, &self.allocator, uniform_buffer_size)?;

        self.depth_stencil_image_format = Some(depth_stencil_image_format);
        self.depth_stencil_image = Some(depth_stencil_image);
        self.depth_stencil_image_view = Some(depth_stencil_image_view);
        self.uniform_buffers = Some(uniform_buffers);

        Ok(())
    }

    pub fn destroy(mut self, neuclidio_window: &NeuclidioWindow) {
        debug!(
            "Destroying Vulkan render buffers for window with id: {:?}",
            neuclidio_window.id
        );
        for render_buffer in self.render_buffers.values() {
            render_buffer.virtual_block.clear();
            unsafe {
                self.allocator
                    .destroy_buffer(render_buffer.buffer, render_buffer.allocation);
            }
        }
        self.render_buffers.clear();

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

        if let Some(depth_stencil_image_view) = self.depth_stencil_image_view.take() {
            debug!(
                "Destroying Vulkan depth image view for window with id: {:?}",
                neuclidio_window.id
            );

            unsafe {
                neuclidio_window
                    .logical_device
                    .destroy_image_view(depth_stencil_image_view, None);
            }
        }

        if let Some(depth_stencil_image) = self.depth_stencil_image.take() {
            debug!(
                "Destroying Vulkan depth image for window with id: {:?}",
                neuclidio_window.id
            );

            unsafe {
                self.allocator
                    .destroy_image(depth_stencil_image.0, depth_stencil_image.1);
            }
        }

        debug!(
            "Destroying Vulkan Memory Allocator for window with id: {:?}",
            neuclidio_window.id
        );

        drop(self.allocator);
    }

    pub fn submit_renderables(
        &mut self,
        neuclidio_window: &NeuclidioWindow,
        command_state: &RenderPipelineCommandState,
        renderables: &[Renderable],
    ) -> NeuclidioResult<()> {
        for renderable in renderables {
            self.allocate_renderable(neuclidio_window, renderable)?;
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

                self.copy_to_render_buffer(neuclidio_window, command_state, uncopied_renderables)?;
                uncopied_renderables = vec![];

                continue;
            }

            uncopied_renderables.push(renderable);
        }

        if !uncopied_renderables.is_empty() {
            self.copy_to_render_buffer(neuclidio_window, command_state, uncopied_renderables)?;
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
        neuclidio_window: &NeuclidioWindow,
        synchronization_state: &RenderPipelineSynchronizationState,
    ) -> NeuclidioResult<()> {
        let frame_index_semaphore_value =
            synchronization_state.frame_index_semaphore_value(neuclidio_window)?;

        let mut renderables_to_deallocate = vec![];
        for (renderable_index, renderable) in
            self.renderables_pending_deallocation.iter().enumerate()
        {
            if let Some(last_used_in_frame) = renderable.last_used_in_frame() {
                let last_used_in_frame = last_used_in_frame.lock().unwrap();

                if let Some(last_used_in_frame) = *last_used_in_frame
                    && last_used_in_frame > frame_index_semaphore_value
                {
                    continue;
                }

                renderables_to_deallocate.push(renderable_index);

                continue;
            }

            warn!(
                "Tried to deallocate a renderable that was never used in the render pipeline for window with id: {:?}",
                neuclidio_window.id
            );
        }

        for renderable_index in renderables_to_deallocate.into_iter().rev() {
            let renderable = self
                .renderables_pending_deallocation
                .remove(renderable_index);
            self.deallocate_renderable(neuclidio_window, &renderable)?;
        }

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

    pub fn depth_stencil_image_format(&self) -> NeuclidioResult<vk::Format> {
        self.depth_stencil_image_format
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    pub fn depth_stencil_image_view(&self) -> NeuclidioResult<vk::ImageView> {
        self.depth_stencil_image_view
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    pub fn render_buffer(&self, render_buffer_id: RenderBufferId) -> Option<vk::Buffer> {
        self.render_buffers
            .get(&render_buffer_id)
            .map(|render_buffer| render_buffer.buffer)
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

    fn get_image_format(
        neuclidio_window: &NeuclidioWindow,
        image_tiling: vk::ImageTiling,
        format_features: vk::FormatFeatureFlags,
        preferred_formats: &[vk::Format],
    ) -> NeuclidioResult<vk::Format> {
        for preferred_format in preferred_formats.iter() {
            let properties = unsafe {
                neuclidio_window
                    .instance
                    .get_physical_device_format_properties(
                        neuclidio_window.physical_device,
                        *preferred_format,
                    )
            };

            match image_tiling {
                vk::ImageTiling::LINEAR => {
                    if properties.linear_tiling_features.contains(format_features) {
                        return Ok(*preferred_format);
                    }
                }
                vk::ImageTiling::OPTIMAL => {
                    if properties.optimal_tiling_features.contains(format_features) {
                        return Ok(*preferred_format);
                    }
                }
                _ => {}
            }
        }

        Err(RenderError::MissingImageFormat.into())
    }

    fn create_depth_stencil_image(
        neuclidio_window: &NeuclidioWindow,
        allocator: &Allocator,
        image_format: vk::Format,
        image_tiling: vk::ImageTiling,
        image_usage: vk::ImageUsageFlags,
    ) -> NeuclidioResult<(vk::Image, Allocation)> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        let image_extent = vk::Extent3D::builder()
            .width(swap_chain.extent().width)
            .height(swap_chain.extent().height)
            .depth(1)
            .build();

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

        let depth_stencil_image =
            unsafe { allocator.create_image(image_create_info, &allocation_options)? };

        Ok(depth_stencil_image)
    }

    fn create_depth_stencil_image_view(
        neuclidio_window: &NeuclidioWindow,
        image: vk::Image,
        image_format: vk::Format,
    ) -> NeuclidioResult<vk::ImageView> {
        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::_2D)
            .format(image_format)
            .subresource_range(subresource_range);

        let depth_stencil_image_view = unsafe {
            neuclidio_window
                .logical_device
                .create_image_view(&image_view_create_info, None)?
        };

        Ok(depth_stencil_image_view)
    }

    fn allocate_renderable(
        &mut self,
        neuclidio_window: &NeuclidioWindow,
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
                    last_used_in_frame: Arc::new(Mutex::new(None)),
                };

                renderable.set_memory_allocation(Some(renderable_memory_allocation));
                render_buffer.renderable_count += 1;

                return Ok(());
            }
        }

        let render_buffer_id = IdGenerator::generate_render_buffer_id();

        debug!(
            "Creating Vulkan render buffer with id '{render_buffer_id:?}' for window with id: {:?}",
            neuclidio_window.id
        );

        let virtual_block_size = max(virtual_allocation_size, MIN_RENDER_BUFFER_SIZE);
        let virtual_block_options = VirtualBlockOptions {
            size: virtual_block_size,
            flags: VirtualBlockCreateFlags::empty(),
        };

        let (buffer, allocation) = create_buffer(
            &self.allocator,
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
            last_used_in_frame: Arc::new(Mutex::new(None)),
        };

        renderable.set_memory_allocation(Some(renderable_memory_allocation));

        self.render_buffers.insert(render_buffer_id, render_buffer);

        Ok(())
    }

    fn deallocate_renderable(
        &mut self,
        neuclidio_window: &NeuclidioWindow,
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
                    debug!(
                        "Destroying Vulkan render buffer with id '{render_buffer_id:?}' for window with id: {:?}",
                        neuclidio_window.id
                    );

                    unsafe {
                        self.allocator
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
        neuclidio_window: &NeuclidioWindow,
        command_state: &RenderPipelineCommandState,
        renderables: Vec<&Renderable>,
    ) -> NeuclidioResult<()> {
        trace!(
            "Creating Vulkan staging render buffer and copying {} renderables with it for window with id: {:?}",
            renderables.len(),
            neuclidio_window.id
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
            &self.allocator,
            staging_render_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryUsage::AutoPreferHost,
            AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
        )?;

        let staging_render_buffer_memory: *mut u8 =
            unsafe { self.allocator.map_memory(staging_render_buffer.1)? };

        for renderable in renderables {
            renderable.load_into_staging_render_buffer(staging_render_buffer_memory);
        }

        unsafe {
            self.allocator.unmap_memory(staging_render_buffer.1);
            self.allocator
                .flush_allocation(staging_render_buffer.1, 0, vk::WHOLE_SIZE)?;
        }

        copy_buffer(
            neuclidio_window,
            command_state.command_pool(),
            staging_render_buffer_size,
            staging_render_buffer.0,
            render_buffer,
            0,
            render_buffer_offset,
        )?;

        unsafe {
            self.allocator
                .destroy_buffer(staging_render_buffer.0, staging_render_buffer.1);
        }

        Ok(())
    }
}
