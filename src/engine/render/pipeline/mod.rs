use crate::engine::render::pipeline::standard::StandardRenderPipeline;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, Handle, HasBuilder};
use vulkanalia_vma::{
    Alloc, Allocation, AllocationCreateFlags, AllocationOptions, Allocator, MemoryUsage,
};

pub mod error;

pub(crate) mod push_constant;
pub(crate) mod standard;
pub(crate) mod state;
pub(crate) mod uniform;
pub(crate) mod vertex;

pub trait RenderPipelineExt {
    fn render(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()>;
    fn prepare_for_reset(&mut self, neuclidio_window: &NeuclidioWindow);
    fn reset(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()>;
    fn destroy(self, neuclidio_window: &NeuclidioWindow);
}

pub enum RenderPipeline {
    Standard(Box<StandardRenderPipeline>),
}

impl RenderPipelineExt for RenderPipeline {
    fn render(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()> {
        match self {
            RenderPipeline::Standard(pipeline) => pipeline.render(neuclidio_window),
        }
    }

    fn prepare_for_reset(&mut self, neuclidio_window: &NeuclidioWindow) {
        match self {
            RenderPipeline::Standard(pipeline) => pipeline.prepare_for_reset(neuclidio_window),
        }
    }

    fn reset(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()> {
        match self {
            RenderPipeline::Standard(pipeline) => pipeline.reset(neuclidio_window),
        }
    }

    fn destroy(self, neuclidio_window: &NeuclidioWindow) {
        match self {
            RenderPipeline::Standard(pipeline) => pipeline.destroy(neuclidio_window),
        }
    }
}

pub(crate) fn create_buffer(
    allocator: &Allocator,
    size: vk::DeviceSize,
    buffer_usage: vk::BufferUsageFlags,
    memory_usage: MemoryUsage,
    allocation_flags: AllocationCreateFlags,
) -> NeuclidioResult<(vk::Buffer, Allocation)> {
    let buffer_create_info = vk::BufferCreateInfo::builder()
        .size(size)
        .usage(buffer_usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .flags(vk::BufferCreateFlags::empty())
        .build();

    let mut allocation_options = AllocationOptions::default();
    allocation_options.usage = memory_usage;
    allocation_options.flags = allocation_flags;

    let buffer_allocation =
        unsafe { allocator.create_buffer(buffer_create_info, &allocation_options)? };
    Ok(buffer_allocation)
}

pub(crate) fn copy_buffer(
    neuclidio_window: &NeuclidioWindow,
    command_pool: vk::CommandPool,
    size: vk::DeviceSize,
    source: vk::Buffer,
    destination: vk::Buffer,
) -> NeuclidioResult<()> {
    let logical_device = &neuclidio_window.logical_device;
    let graphics_queue = neuclidio_window.graphics_queue;

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

    let buffer_copy = vk::BufferCopy::builder().size(size).build();

    unsafe {
        logical_device.cmd_copy_buffer(command_buffer, source, destination, &[buffer_copy]);
        logical_device.end_command_buffer(command_buffer)?;
    }

    let command_buffers = &[command_buffer];
    let submit_info = vk::SubmitInfo::builder()
        .command_buffers(command_buffers)
        .build();

    unsafe {
        logical_device.queue_submit(graphics_queue, &[submit_info], vk::Fence::null())?;
        logical_device.queue_wait_idle(graphics_queue)?;
        logical_device.free_command_buffers(command_pool, command_buffers);
    }

    Ok(())
}
