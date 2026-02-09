use crate::engine::render::error::RenderError;
use crate::engine::render::pipeline::common::state::command::RenderPipelineCommandState;
use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::pipeline::{copy_buffer, create_buffer};
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use log::{debug, warn};
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, HasBuilder, InstanceV1_0};
use vulkanalia_vma::{
    Alloc, Allocation, AllocationCreateFlags, AllocationOptions, Allocator, AllocatorOptions,
    MemoryUsage,
};

pub struct RenderPipelineAllocatorState {
    allocator: Allocator,
    depth_stencil_image_format: Option<vk::Format>,
    depth_stencil_image: Option<(vk::Image, Allocation)>,
    depth_stencil_image_view: Option<vk::ImageView>,
    render_buffers: Option<Vec<Option<(vk::Buffer, Allocation)>>>,
    render_buffer_stale_state: Option<Vec<bool>>,
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
            depth_stencil_image_format: None,
            depth_stencil_image: None,
            depth_stencil_image_view: None,
            render_buffers: None,
            render_buffer_stale_state: None,
            uniform_buffers: None,
        })
    }

    pub fn prepare_for_reset(&mut self, neuclidio_window: &NeuclidioWindow) {
        if let Some(render_buffers) = self.render_buffers.take() {
            debug!(
                "Destroying Vulkan render buffers for window with id: {:?}",
                neuclidio_window.id
            );
            for render_buffer in render_buffers.iter() {
                if let Some(render_buffer) = render_buffer {
                    unsafe {
                        self.allocator
                            .destroy_buffer(render_buffer.0, render_buffer.1);
                    }
                }
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

        let (render_buffers, render_buffer_stale_state) =
            Self::create_render_buffers(neuclidio_window)?;

        debug!(
            "Creating Vulkan uniform buffers for window with id: {:?}",
            neuclidio_window.id
        );

        let uniform_buffers =
            Self::create_uniform_buffers(neuclidio_window, &self.allocator, uniform_buffer_size)?;

        self.depth_stencil_image_format = Some(depth_stencil_image_format);
        self.depth_stencil_image = Some(depth_stencil_image);
        self.depth_stencil_image_view = Some(depth_stencil_image_view);
        self.render_buffers = Some(render_buffers);
        self.render_buffer_stale_state = Some(render_buffer_stale_state);
        self.uniform_buffers = Some(uniform_buffers);

        Ok(())
    }

    pub fn destroy(mut self, neuclidio_window: &NeuclidioWindow) {
        if let Some(render_buffers) = self.render_buffers.take() {
            debug!(
                "Destroying Vulkan render buffers for window with id: {:?}",
                neuclidio_window.id
            );
            for render_buffer in render_buffers.iter() {
                if let Some(render_buffer) = render_buffer {
                    unsafe {
                        self.allocator
                            .destroy_buffer(render_buffer.0, render_buffer.1);
                    }
                }
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

    pub fn mark_render_buffers_as_stale(&mut self) {
        if let Some(render_buffer_stale_state) = self.render_buffer_stale_state.as_mut() {
            for is_stale in render_buffer_stale_state.iter_mut() {
                *is_stale = true;
            }
        }
    }

    pub fn fill_render_buffer<RBF>(
        &mut self,
        neuclidio_window: &NeuclidioWindow,
        render_buffer_index: usize,
        command_state: &RenderPipelineCommandState,
        render_buffer_size: vk::DeviceSize,
        mut render_buffer_filler: RBF,
    ) -> NeuclidioResult<()>
    where
        RBF: FnMut(*mut u8),
    {
        if render_buffer_size == 0 {
            return Ok(());
        }

        match self.render_buffer_stale_state.as_ref() {
            Some(render_buffer_stale_state) => {
                if let Some(is_stale) = render_buffer_stale_state.get(render_buffer_index) {
                    if !is_stale {
                        return Ok(());
                    }
                } else {
                    warn!("Render buffer index '{render_buffer_index}' out of bounds");
                    return Ok(());
                }
            }
            None => return Ok(()),
        }

        let render_buffer = match self.render_buffers.as_mut() {
            Some(render_buffers) => {
                if let Some(render_buffer) = render_buffers.get_mut(render_buffer_index) {
                    render_buffer
                } else {
                    warn!("Render buffer index '{render_buffer_index}' out of bounds");
                    return Ok(());
                }
            }
            None => return Ok(()),
        };

        if let Some(render_buffer) = render_buffer.take() {
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

        self.render_buffers.as_mut().unwrap()[render_buffer_index] = Some(render_buffer);
        self.render_buffer_stale_state.as_mut().unwrap()[render_buffer_index] = false;

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

    pub fn render_buffer(&self, render_buffer_index: usize) -> Option<vk::Buffer> {
        self.render_buffers
            .as_ref()
            .and_then(|render_buffers| render_buffers.get(render_buffer_index))
            .and_then(|render_buffer| render_buffer.map(|render_buffer| render_buffer.0))
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

    fn create_render_buffers(
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<(Vec<Option<(vk::Buffer, Allocation)>>, Vec<bool>)> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        let render_buffers = vec![None; swap_chain.image_count()];
        let render_buffer_stale_state = vec![true; swap_chain.image_count()];

        Ok((render_buffers, render_buffer_stale_state))
    }
}
