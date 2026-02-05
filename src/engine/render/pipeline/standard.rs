use crate::engine::render::error::NeuclidioRenderError;
use crate::engine::render::pipeline::synchronization::NeuclidioRenderPipelineSynchronization;
use crate::engine::render::pipeline::{
    NeuclidioRenderPipeline, create_shader_module, destroy_shader_module,
};
use crate::engine::render::windowing::queue_family_indices::QueueFamilyIndices;
use crate::engine::render::windowing::swap_chain::SwapChain;
use crate::error::NeuclidioResult;
use log::debug;
use vulkanalia::vk::{DeviceV1_0, Handle, HasBuilder, KhrSwapchainExtensionDeviceCommands};
use vulkanalia::{Device, vk};

const VERTEX_SHADER_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/standard/vertex_shader.spv"
));
const FRAGMENT_SHADER_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/standard/fragment_shader.spv"
));

pub struct NeuclidioStandardRenderPipeline {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    frame_buffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    synchronization: NeuclidioRenderPipelineSynchronization,
}

impl NeuclidioStandardRenderPipeline {}

impl NeuclidioStandardRenderPipeline {
    pub fn new(
        logical_device: &Device,
        swap_chain: &SwapChain,
        queue_family_indices: QueueFamilyIndices,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        debug!("Creating Vulkan pipeline for a standard Neuclidio render pipeline");
        let (pipeline, pipeline_layout, render_pass, frame_buffers) =
            Self::create_pipeline(logical_device, swap_chain)?;

        debug!("Creating Vulkan command pool for a standard Neuclidio render pipeline");
        let command_pool = Self::create_command_pool(logical_device, queue_family_indices)?;

        debug!("Creating Vulkan command buffers for a standard Neuclidio render pipeline");
        let command_buffers = Self::create_command_buffers(
            logical_device,
            swap_chain,
            render_pass,
            pipeline,
            command_pool,
            &frame_buffers,
        )?;

        debug!("Creating Vulkan synchronization objects for a standard Neuclidio render pipeline");
        let synchronization =
            Self::create_synchronization(logical_device, swap_chain, max_frames_in_flight)?;

        Ok(Self {
            pipeline,
            pipeline_layout,
            render_pass,
            frame_buffers,
            command_pool,
            command_buffers,
            synchronization,
        })
    }

    fn create_pipeline(
        logical_device: &Device,
        swap_chain: &SwapChain,
    ) -> NeuclidioResult<(
        vk::Pipeline,
        vk::PipelineLayout,
        vk::RenderPass,
        Vec<vk::Framebuffer>,
    )> {
        let vertex_shader_module = create_shader_module(logical_device, VERTEX_SHADER_BYTECODE)?;
        let fragment_shader_module =
            create_shader_module(logical_device, FRAGMENT_SHADER_BYTECODE)?;

        let vertex_shader_stage = Self::create_vertex_shader_stage(vertex_shader_module);
        let fragment_shader_stage = Self::create_fragment_shader_stage(fragment_shader_module);
        let vertex_input_state = Self::create_vertex_input_state();
        let input_assembly_state = Self::create_input_assembly_state();
        let viewport_state = Self::create_viewport_state(swap_chain);
        let rasterization_state = Self::create_rasterization_state();
        let multisample_state = Self::create_multisample_state();
        let color_blend_state = Self::create_color_blend_state();

        let pipeline_layout = Self::create_pipeline_layout(logical_device)?;
        let render_pass = Self::create_render_pass(logical_device, swap_chain.image_format)?;

        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&[vertex_shader_stage, fragment_shader_stage])
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state.0)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state.0)
            .layout(pipeline_layout)
            .render_pass(render_pass.0)
            .build();

        let pipeline = unsafe {
            logical_device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_create_info],
                None,
            )?
        }
        .0[0];

        destroy_shader_module(logical_device, vertex_shader_module);
        destroy_shader_module(logical_device, fragment_shader_module);

        let render_pass = render_pass.0;
        let frame_buffers = Self::create_frame_buffers(logical_device, swap_chain, render_pass)?;

        Ok((pipeline, pipeline_layout, render_pass, frame_buffers))
    }

    fn create_vertex_shader_stage(
        vertex_shader_module: vk::ShaderModule,
    ) -> vk::PipelineShaderStageCreateInfo {
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader_module)
            .name(b"main\0")
            .build()
    }

    fn create_fragment_shader_stage(
        fragment_shader_module: vk::ShaderModule,
    ) -> vk::PipelineShaderStageCreateInfo {
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_shader_module)
            .name(b"main\0")
            .build()
    }

    fn create_vertex_input_state() -> vk::PipelineVertexInputStateCreateInfo {
        vk::PipelineVertexInputStateCreateInfo::builder().build()
    }

    fn create_input_assembly_state() -> vk::PipelineInputAssemblyStateCreateInfo {
        vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .build()
    }

    fn create_viewport_state(
        swap_chain: &SwapChain,
    ) -> (
        vk::PipelineViewportStateCreateInfo,
        Vec<vk::Viewport>,
        Vec<vk::Rect2D>,
    ) {
        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(swap_chain.extent.width as f32)
            .height(swap_chain.extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build();

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(swap_chain.extent)
            .build();

        let viewports = vec![viewport];
        let scissors = vec![scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors)
            .build();

        (viewport_state, viewports, scissors)
    }

    fn create_rasterization_state() -> vk::PipelineRasterizationStateCreateInfo {
        vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0)
            .build()
    }

    fn create_multisample_state() -> vk::PipelineMultisampleStateCreateInfo {
        vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::_1)
            .build()
    }

    fn create_color_blend_state() -> (
        vk::PipelineColorBlendStateCreateInfo,
        Vec<vk::PipelineColorBlendAttachmentState>,
    ) {
        let attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(false)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .build();

        let attachments = vec![attachment];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&attachments)
            .blend_constants([0.0; 4])
            .build();

        (color_blend_state, attachments)
    }

    fn create_pipeline_layout(logical_device: &Device) -> NeuclidioResult<vk::PipelineLayout> {
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder().build();

        let pipeline_layout =
            unsafe { logical_device.create_pipeline_layout(&pipeline_layout_create_info, None)? };

        Ok(pipeline_layout)
    }

    fn create_render_pass(
        logical_device: &Device,
        color_attachment_format: vk::Format,
    ) -> NeuclidioResult<(
        vk::RenderPass,
        Vec<vk::AttachmentDescription>,
        Vec<vk::SubpassDescription>,
    )> {
        let color_attachment = vk::AttachmentDescription::builder()
            .format(color_attachment_format)
            .samples(vk::SampleCountFlags::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();

        let subpass = {
            let color_attachment_reference = vk::AttachmentReference::builder()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .build();

            vk::SubpassDescription::builder()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&[color_attachment_reference])
                .build()
        };

        let attachments = vec![color_attachment];
        let subpasses = vec![subpass];

        let render_pass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .build();

        let render_pass =
            unsafe { logical_device.create_render_pass(&render_pass_create_info, None)? };

        Ok((render_pass, attachments, subpasses))
    }

    fn create_frame_buffers(
        logical_device: &Device,
        swap_chain: &SwapChain,
        render_pass: vk::RenderPass,
    ) -> NeuclidioResult<Vec<vk::Framebuffer>> {
        let mut frame_buffers = Vec::with_capacity(swap_chain.image_views.len());

        for image_view in swap_chain.image_views.iter() {
            let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&[*image_view])
                .width(swap_chain.extent.width)
                .height(swap_chain.extent.height)
                .layers(1)
                .build();

            let frame_buffer =
                unsafe { logical_device.create_framebuffer(&frame_buffer_create_info, None)? };

            frame_buffers.push(frame_buffer);
        }

        Ok(frame_buffers)
    }

    fn create_command_pool(
        logical_device: &Device,
        queue_family_indices: QueueFamilyIndices,
    ) -> NeuclidioResult<vk::CommandPool> {
        let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::empty())
            .queue_family_index(queue_family_indices.graphics)
            .build();

        let command_pool =
            unsafe { logical_device.create_command_pool(&command_pool_create_info, None)? };

        Ok(command_pool)
    }

    fn create_command_buffers(
        logical_device: &Device,
        swap_chain: &SwapChain,
        render_pass: vk::RenderPass,
        pipeline: vk::Pipeline,
        command_pool: vk::CommandPool,
        frame_buffers: &[vk::Framebuffer],
    ) -> NeuclidioResult<Vec<vk::CommandBuffer>> {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(frame_buffers.len() as u32);

        let command_buffers =
            unsafe { logical_device.allocate_command_buffers(&command_buffer_allocate_info)? };

        for (i, command_buffer) in command_buffers.iter().enumerate() {
            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder().build();

            unsafe {
                logical_device.begin_command_buffer(*command_buffer, &command_buffer_begin_info)?;

                let render_area = vk::Rect2D::builder()
                    .offset(vk::Offset2D::default())
                    .extent(swap_chain.extent)
                    .build();

                let clear_value = vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                };

                let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                    .render_pass(render_pass)
                    .framebuffer(frame_buffers[i])
                    .render_area(render_area)
                    .clear_values(&[clear_value])
                    .build();

                logical_device.cmd_begin_render_pass(
                    *command_buffer,
                    &render_pass_begin_info,
                    vk::SubpassContents::INLINE,
                );
                logical_device.cmd_bind_pipeline(
                    *command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline,
                );
                logical_device.cmd_draw(*command_buffer, 3, 1, 0, 0);
                logical_device.cmd_end_render_pass(*command_buffer);

                logical_device.end_command_buffer(*command_buffer)?;
            }
        }

        Ok(command_buffers)
    }

    fn create_synchronization(
        logical_device: &Device,
        swap_chain: &SwapChain,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<NeuclidioRenderPipelineSynchronization> {
        let mut image_available_semaphores = Vec::with_capacity(max_frames_in_flight);
        let mut render_finished_semaphores = Vec::with_capacity(max_frames_in_flight);
        let mut in_flight_fences = Vec::with_capacity(max_frames_in_flight);
        let mut images_in_flight = Vec::with_capacity(swap_chain.images.len());

        for _ in 0..max_frames_in_flight {
            image_available_semaphores.push(Self::create_semaphore(logical_device)?);
            render_finished_semaphores.push(Self::create_semaphore(logical_device)?);
            in_flight_fences.push(Self::create_fence(logical_device)?);
        }

        for _ in 0..swap_chain.images.len() {
            images_in_flight.push(vk::Fence::null());
        }

        let synchronization = NeuclidioRenderPipelineSynchronization {
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
            max_frames_in_flight,
            frame: 0,
        };

        Ok(synchronization)
    }

    fn create_semaphore(logical_device: &Device) -> NeuclidioResult<vk::Semaphore> {
        let semaphore_create_info = vk::SemaphoreCreateInfo::builder().build();

        let semaphore = unsafe { logical_device.create_semaphore(&semaphore_create_info, None)? };

        Ok(semaphore)
    }

    fn create_fence(logical_device: &Device) -> NeuclidioResult<vk::Fence> {
        let fence_create_info = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED)
            .build();

        let fence = unsafe { logical_device.create_fence(&fence_create_info, None)? };

        Ok(fence)
    }
}

impl NeuclidioRenderPipeline for NeuclidioStandardRenderPipeline {
    fn render(
        &mut self,
        logical_device: &Device,
        swap_chain: &SwapChain,
        graphics_queue: vk::Queue,
        present_queue: vk::Queue,
    ) -> NeuclidioResult<()> {
        self.synchronization
            .wait_for_in_flight_fence(logical_device)?;

        let image_index_result = unsafe {
            logical_device.acquire_next_image_khr(
                swap_chain.chain,
                u64::MAX,
                self.synchronization.get_current_image_available_semaphore(),
                vk::Fence::null(),
            )
        };

        let image_index = match image_index_result {
            Ok((image_index, _)) => image_index as usize,
            Err(vk::ErrorCode::OUT_OF_DATE_KHR) => {
                return Err(NeuclidioRenderError::OutOfDateSwapChain.into());
            }
            Err(err) => return Err(err.into()),
        };

        self.synchronization
            .wait_for_image_in_flight(logical_device, image_index)?;
        self.synchronization
            .set_image_in_flight_to_current_in_flight_fence(image_index)?;
        self.synchronization
            .reset_current_in_flight_fence(logical_device)?;

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&[self.synchronization.get_current_image_available_semaphore()])
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&[self.command_buffers[image_index]])
            .signal_semaphores(&[self.synchronization.get_current_render_finished_semaphore()])
            .build();

        unsafe {
            logical_device.queue_submit(
                graphics_queue,
                &[submit_info],
                self.synchronization.get_current_in_flight_fence(),
            )?;
        }

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&[self.synchronization.get_current_render_finished_semaphore()])
            .swapchains(&[swap_chain.chain])
            .image_indices(&[image_index as u32])
            .build();

        unsafe {
            logical_device.queue_present_khr(present_queue, &present_info)?;
        }

        self.synchronization.increment_frame();

        Ok(())
    }

    fn recreate_frame_buffers(
        &mut self,
        logical_device: &Device,
        swap_chain: &SwapChain,
    ) -> NeuclidioResult<()> {
        debug!("Re-creating Vulkan pipeline for a standard Neuclidio render pipeline");
        let (pipeline, pipeline_layout, render_pass, frame_buffers) =
            Self::create_pipeline(logical_device, swap_chain)?;

        debug!("Re-creating Vulkan command buffers for a standard Neuclidio render pipeline");
        let command_buffers = Self::create_command_buffers(
            logical_device,
            swap_chain,
            render_pass,
            pipeline,
            self.command_pool,
            &frame_buffers,
        )?;

        self.pipeline = pipeline;
        self.pipeline_layout = pipeline_layout;
        self.render_pass = render_pass;
        self.frame_buffers = frame_buffers;
        self.command_buffers = command_buffers;

        Ok(())
    }

    fn destroy_frame_buffers(&self, logical_device: &Device, skip_freeing_command_buffers: bool) {
        debug!("Destroying Vulkan frame buffers for a standard Neuclidio render pipeline");
        for frame_buffer in self.frame_buffers.iter() {
            unsafe {
                logical_device.destroy_framebuffer(*frame_buffer, None);
            }
        }

        if !skip_freeing_command_buffers {
            debug!("Freeing Vulkan command buffers for a standard Neuclidio render pipeline");
            unsafe {
                logical_device.free_command_buffers(self.command_pool, &self.command_buffers);
            }
        }

        debug!("Destroying Vulkan pipeline for a standard Neuclidio render pipeline");
        unsafe {
            logical_device.destroy_pipeline(self.pipeline, None);
        }

        debug!("Destroying Vulkan pipeline layout for a standard Neuclidio render pipeline");
        unsafe {
            logical_device.destroy_pipeline_layout(self.pipeline_layout, None);
        }

        debug!("Destroying Vulkan render pass for a standard Neuclidio render pipeline");
        unsafe {
            logical_device.destroy_render_pass(self.render_pass, None);
        }
    }

    fn destroy(&self, logical_device: &Device) {
        debug!("Destroying Vulkan fences for a standard Neuclidio render pipeline");
        for fence in self.synchronization.in_flight_fences.iter() {
            unsafe {
                logical_device.destroy_fence(*fence, None);
            }
        }

        debug!("Destroying Vulkan semaphores for a standard Neuclidio render pipeline");
        for semaphore in self.synchronization.render_finished_semaphores.iter() {
            unsafe {
                logical_device.destroy_semaphore(*semaphore, None);
            }
        }
        for semaphore in self.synchronization.image_available_semaphores.iter() {
            unsafe {
                logical_device.destroy_semaphore(*semaphore, None);
            }
        }

        debug!("Destroying Vulkan command pool for a standard Neuclidio render pipeline");
        unsafe {
            logical_device.destroy_command_pool(self.command_pool, None);
        }
    }
}
