use crate::engine::render::pipeline::standard::StandardRenderPipeline;
use crate::engine::render::renderable::Renderable;
use crate::engine::render::vulkan_context::VulkanContext;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::entity::Entity;
use crate::error::NeuclidioResult;
use vulkanalia::vk;
use vulkanalia::vk::{HasBuilder, InstanceV1_0};
use vulkanalia_vma::{
    Alloc, Allocation, AllocationCreateFlags, AllocationOptions, Allocator, MemoryUsage,
};

pub mod error;

pub(crate) mod common;
pub(crate) mod standard;

pub trait RenderPipelineExt {
    fn submit_entity(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        entity: &Entity,
    ) -> NeuclidioResult<()>;
    fn remove_entity(&mut self, entity: &Entity) -> NeuclidioResult<()>;

    fn handle_renderable_added(
        &mut self,
        vulkan_context: &VulkanContext,
        entity: &Entity,
        renderable: Renderable,
    ) -> NeuclidioResult<()>;
    fn handle_renderable_removed(
        &mut self,
        entity: &Entity,
        renderable: Renderable,
    ) -> NeuclidioResult<()>;

    fn render(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<()>;

    fn prepare_for_window_reset(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    );
    fn reset_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<()>;
    fn clean_up_for_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    );
    fn destroy(self, vulkan_context: &VulkanContext);
}

pub enum RenderPipeline {
    Standard(Box<StandardRenderPipeline>),
}

impl RenderPipelineExt for RenderPipeline {
    fn submit_entity(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        entity: &Entity,
    ) -> NeuclidioResult<()> {
        match self {
            RenderPipeline::Standard(pipeline) => {
                pipeline.submit_entity(vulkan_context, neuclidio_window, entity)
            }
        }
    }

    fn remove_entity(&mut self, entity: &Entity) -> NeuclidioResult<()> {
        match self {
            RenderPipeline::Standard(pipeline) => pipeline.remove_entity(entity),
        }
    }

    fn handle_renderable_added(
        &mut self,
        vulkan_context: &VulkanContext,
        entity: &Entity,
        renderable: Renderable,
    ) -> NeuclidioResult<()> {
        match self {
            RenderPipeline::Standard(pipeline) => {
                pipeline.handle_renderable_added(vulkan_context, entity, renderable)
            }
        }
    }

    fn handle_renderable_removed(
        &mut self,
        entity: &Entity,
        renderable: Renderable,
    ) -> NeuclidioResult<()> {
        match self {
            RenderPipeline::Standard(pipeline) => {
                pipeline.handle_renderable_removed(entity, renderable)
            }
        }
    }

    fn render(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<()> {
        match self {
            RenderPipeline::Standard(pipeline) => pipeline.render(vulkan_context, neuclidio_window),
        }
    }

    fn prepare_for_window_reset(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) {
        match self {
            RenderPipeline::Standard(pipeline) => {
                pipeline.prepare_for_window_reset(vulkan_context, neuclidio_window)
            }
        }
    }

    fn reset_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<()> {
        match self {
            RenderPipeline::Standard(pipeline) => {
                pipeline.reset_window(vulkan_context, neuclidio_window)
            }
        }
    }

    fn clean_up_for_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) {
        match self {
            RenderPipeline::Standard(pipeline) => {
                pipeline.clean_up_for_window(vulkan_context, neuclidio_window)
            }
        }
    }

    fn destroy(self, vulkan_context: &VulkanContext) {
        match self {
            RenderPipeline::Standard(pipeline) => pipeline.destroy(vulkan_context),
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

fn get_supported_image_format(
    vulkan_context: &VulkanContext,
    image_tiling: vk::ImageTiling,
    format_features: vk::FormatFeatureFlags,
    preferred_formats: &[vk::Format],
) -> Option<vk::Format> {
    for preferred_format in preferred_formats.iter() {
        let properties = unsafe {
            vulkan_context
                .instance
                .get_physical_device_format_properties(
                    vulkan_context.physical_device,
                    *preferred_format,
                )
        };

        match image_tiling {
            vk::ImageTiling::LINEAR => {
                if properties.linear_tiling_features.contains(format_features) {
                    return Some(*preferred_format);
                }
            }
            vk::ImageTiling::OPTIMAL => {
                if properties.optimal_tiling_features.contains(format_features) {
                    return Some(*preferred_format);
                }
            }
            _ => {}
        }
    }

    None
}
