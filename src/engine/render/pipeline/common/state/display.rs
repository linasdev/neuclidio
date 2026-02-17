use crate::engine::render::pipeline::common::state::allocator::RenderPipelineAllocatorState;
use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::vulkan_context::VulkanContext;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use log::debug;
use std::collections::HashMap;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::vk::{DeviceV1_0, Handle, HasBuilder};
use vulkanalia::{Device, vk};
use winit::window::WindowId;

pub struct DisplayPipelineState {
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    window_states: HashMap<WindowId, DisplayPipelineWindowState>,
    max_frames_in_flight: usize,
}

impl DisplayPipelineState {
    pub fn new(
        vulkan_context: &VulkanContext,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        debug!("Creating Vulkan descriptor set layout");

        let descriptor_set_layout = Self::create_descriptor_set_layout(vulkan_context)?;

        debug!("Creating Vulkan pipeline layout");

        let descriptor_set_layouts = [descriptor_set_layout];
        let pipeline_layout =
            Self::create_pipeline_layout(vulkan_context, &descriptor_set_layouts)?;

        let window_states = HashMap::new();

        Ok(Self {
            descriptor_set_layout,
            pipeline_layout,
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
        vertex_shader_bytecode: &[u8],
        fragment_shader_bytecode: &[u8],
    ) -> NeuclidioResult<()> {
        if let Some(window_state) = self.window_states.get_mut(&neuclidio_window.id) {
            window_state.reset_window(
                vulkan_context,
                neuclidio_window,
                allocator_state,
                self.pipeline_layout,
                vertex_shader_bytecode,
                fragment_shader_bytecode,
                self.max_frames_in_flight,
            )?;
            return Ok(());
        }

        let mut window_state = DisplayPipelineWindowState::new(
            vulkan_context,
            neuclidio_window,
            allocator_state,
            self.descriptor_set_layout,
            self.max_frames_in_flight,
        )?;
        window_state.reset_window(
            vulkan_context,
            neuclidio_window,
            allocator_state,
            self.pipeline_layout,
            vertex_shader_bytecode,
            fragment_shader_bytecode,
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

        debug!("Destroying Vulkan pipeline layout");

        unsafe {
            vulkan_context
                .logical_device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }

        debug!("Destroying Vulkan descriptor set layout");

        unsafe {
            vulkan_context
                .logical_device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }

    pub fn record_command_buffer(
        &self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        frame_in_flight_index: usize,
        image_index: usize,
    ) -> NeuclidioResult<()> {
        let logical_device = &vulkan_context.logical_device;
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;
        let render_pass = self.render_pass(neuclidio_window)?;
        let pipeline = self.pipeline(neuclidio_window)?;
        let frame_buffer = self.frame_buffer(neuclidio_window, image_index)?;
        let command_buffer = self.command_buffer(neuclidio_window, frame_in_flight_index)?;
        let descriptor_set = self.descriptor_set(neuclidio_window, frame_in_flight_index)?;

        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(swap_chain.extent())
            .build();

        let clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let clear_values = [clear_value];
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass)
            .framebuffer(frame_buffer)
            .render_area(render_area)
            .clear_values(&clear_values)
            .build();

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(swap_chain.extent().width as f32)
            .height(swap_chain.extent().height as f32)
            .build();
        let viewports = [viewport];

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(swap_chain.extent())
            .build();
        let scissors = [scissor];

        unsafe {
            logical_device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;
        }

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .build();

        unsafe {
            logical_device.begin_command_buffer(command_buffer, &command_buffer_begin_info)?;
            logical_device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            logical_device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline,
            );

            logical_device.cmd_set_viewport(command_buffer, 0, &viewports);
            logical_device.cmd_set_scissor(command_buffer, 0, &scissors);

            logical_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );
            logical_device.cmd_draw(command_buffer, 3, 1, 0, 0);
            logical_device.cmd_end_render_pass(command_buffer);
            logical_device.end_command_buffer(command_buffer)?;
        }

        Ok(())
    }

    pub fn command_buffer(
        &self,
        neuclidio_window: &NeuclidioWindow,
        frame_in_flight_index: usize,
    ) -> NeuclidioResult<vk::CommandBuffer> {
        self.window_states
            .get(&neuclidio_window.id)
            .map(|window_state| window_state.command_buffers[frame_in_flight_index])
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    fn pipeline(&self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<vk::Pipeline> {
        self.window_states
            .get(&neuclidio_window.id)
            .and_then(|window_state| window_state.pipeline)
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    fn render_pass(&self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<vk::RenderPass> {
        self.window_states
            .get(&neuclidio_window.id)
            .and_then(|window_state| window_state.render_pass)
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    fn frame_buffer(
        &self,
        neuclidio_window: &NeuclidioWindow,
        image_index: usize,
    ) -> NeuclidioResult<vk::Framebuffer> {
        self.window_states
            .get(&neuclidio_window.id)
            .and_then(|window_state| window_state.frame_buffers.as_ref())
            .map(|frame_buffers| frame_buffers[image_index])
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    fn descriptor_set(
        &self,
        neuclidio_window: &NeuclidioWindow,
        frame_in_flight_index: usize,
    ) -> NeuclidioResult<vk::DescriptorSet> {
        self.window_states
            .get(&neuclidio_window.id)
            .map(|window_state| window_state.descriptor_sets[frame_in_flight_index])
            .ok_or(RenderPipelineError::Unprepared.into())
    }

    fn create_descriptor_set_layout(
        vulkan_context: &VulkanContext,
    ) -> NeuclidioResult<vk::DescriptorSetLayout> {
        let descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        let descriptor_set_layout_bindings = [descriptor_set_layout_binding];
        let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&descriptor_set_layout_bindings)
            .build();

        let descriptor_set_layout = unsafe {
            vulkan_context
                .logical_device
                .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)?
        };

        Ok(descriptor_set_layout)
    }

    fn create_pipeline_layout(
        vulkan_context: &VulkanContext,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
    ) -> NeuclidioResult<vk::PipelineLayout> {
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&descriptor_set_layouts)
            .build();

        let pipeline_layout = unsafe {
            vulkan_context
                .logical_device
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };

        Ok(pipeline_layout)
    }
}

pub struct DisplayPipelineWindowState {
    window_id: WindowId,
    sampler: vk::Sampler,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    pipeline: Option<vk::Pipeline>,
    render_pass: Option<vk::RenderPass>,
    frame_buffers: Option<Vec<vk::Framebuffer>>,
}

impl DisplayPipelineWindowState {
    fn new(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        allocator_state: &RenderPipelineAllocatorState,
        descriptor_set_layout: vk::DescriptorSetLayout,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        let window_id = neuclidio_window.id;

        debug!("Creating Vulkan graphics command pool for window with id: {window_id:?}");

        let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(vulkan_context.queue_family_indices.graphics)
            .build();

        let sampler = Self::create_sampler(vulkan_context)?;

        let command_pool = unsafe {
            vulkan_context
                .logical_device
                .create_command_pool(&command_pool_create_info, None)?
        };

        let command_buffers = Self::create_command_buffers(
            vulkan_context,
            neuclidio_window,
            command_pool,
            max_frames_in_flight,
        )?;

        let descriptor_pool = Self::create_descriptor_pool(vulkan_context, max_frames_in_flight)?;

        let descriptor_sets = Self::create_descriptor_sets(
            vulkan_context,
            neuclidio_window,
            allocator_state,
            sampler,
            descriptor_set_layout,
            descriptor_pool,
            max_frames_in_flight,
        )?;

        Ok(Self {
            window_id,
            sampler,
            command_pool,
            command_buffers,
            descriptor_pool,
            descriptor_sets,
            pipeline: None,
            render_pass: None,
            frame_buffers: None,
        })
    }

    fn prepare_for_window_reset(&mut self, vulkan_context: &VulkanContext) {
        let logical_device = &vulkan_context.logical_device;

        if let Some(frame_buffers) = self.frame_buffers.take() {
            debug!(
                "Destroying Vulkan frame buffers for window with id: {:?}",
                self.window_id
            );

            for frame_buffer in frame_buffers.into_iter() {
                unsafe {
                    logical_device.destroy_framebuffer(frame_buffer, None);
                }
            }
        }

        if let Some(pipeline) = self.pipeline.take() {
            debug!(
                "Destroying Vulkan pipeline for window with id: {:?}",
                self.window_id
            );

            unsafe {
                logical_device.destroy_pipeline(pipeline, None);
            }
        }

        if let Some(render_pass) = self.render_pass.take() {
            debug!(
                "Destroying Vulkan render pass for window with id: {:?}",
                self.window_id
            );

            unsafe {
                logical_device.destroy_render_pass(render_pass, None);
            }
        }
    }

    fn reset_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        allocator_state: &RenderPipelineAllocatorState,
        pipeline_layout: vk::PipelineLayout,
        vertex_shader_bytecode: &[u8],
        fragment_shader_bytecode: &[u8],
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<()> {
        let logical_device = &vulkan_context.logical_device;

        let vertex_shader_module =
            Self::create_shader_module(logical_device, vertex_shader_bytecode)?;
        let fragment_shader_module =
            Self::create_shader_module(logical_device, fragment_shader_bytecode)?;

        let vertex_shader_stage = Self::create_vertex_shader_stage(vertex_shader_module);
        let fragment_shader_stage = Self::create_fragment_shader_stage(fragment_shader_module);
        let vertex_input_state = Self::create_vertex_input_state();
        let input_assembly_state = Self::create_input_assembly_state();
        let viewport_state = Self::create_viewport_state();
        let rasterization_state = Self::create_rasterization_state();
        let multisample_state = Self::create_multisample_state();
        let color_blend_state = Self::create_color_blend_state();
        let dynamic_state = Self::create_dynamic_state();

        debug!("Creating Vulkan render pass");

        let render_pass = Self::create_render_pass(vulkan_context, neuclidio_window)?;

        debug!("Creating Vulkan pipeline");

        let stages = [vertex_shader_stage, fragment_shader_stage];
        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state.0)
            .dynamic_state(&dynamic_state.0)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .build();

        let pipeline = unsafe {
            logical_device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_create_info],
                None,
            )?
        }
        .0[0];

        Self::destroy_shader_module(logical_device, vertex_shader_module);
        Self::destroy_shader_module(logical_device, fragment_shader_module);

        debug!(
            "Creating Vulkan frame buffers for window with id: {:?}",
            self.window_id
        );

        let frame_buffers =
            Self::create_frame_buffers(vulkan_context, neuclidio_window, render_pass)?;

        self.pipeline = Some(pipeline);
        self.render_pass = Some(render_pass);
        self.frame_buffers = Some(frame_buffers);

        Self::update_descriptor_sets(
            vulkan_context,
            neuclidio_window,
            allocator_state,
            self.sampler,
            &self.descriptor_sets,
            max_frames_in_flight,
        )?;

        Ok(())
    }

    fn destroy(mut self, vulkan_context: &VulkanContext) {
        self.prepare_for_window_reset(vulkan_context);

        debug!(
            "Destroying Vulkan descriptor pool for window with id: {:?}",
            self.window_id
        );

        unsafe {
            vulkan_context
                .logical_device
                .destroy_descriptor_pool(self.descriptor_pool, None);
        }

        debug!(
            "Destroying Vulkan graphics command pool for window with id: {:?}",
            self.window_id
        );

        unsafe {
            vulkan_context
                .logical_device
                .destroy_command_pool(self.command_pool, None);
        }

        debug!(
            "Destroying Vulkan sampler for window with id: {:?}",
            self.window_id
        );

        unsafe {
            vulkan_context
                .logical_device
                .destroy_sampler(self.sampler, None);
        }
    }

    fn create_frame_buffers(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        render_pass: vk::RenderPass,
    ) -> NeuclidioResult<Vec<vk::Framebuffer>> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        let mut frame_buffers = Vec::with_capacity(swap_chain.image_count());

        for swap_chain_image_view in swap_chain.image_views() {
            let attachments = [*swap_chain_image_view];
            let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(swap_chain.extent().width)
                .height(swap_chain.extent().height)
                .layers(1)
                .build();

            let frame_buffer = unsafe {
                vulkan_context
                    .logical_device
                    .create_framebuffer(&frame_buffer_create_info, None)?
            };

            frame_buffers.push(frame_buffer);
        }

        Ok(frame_buffers)
    }

    fn create_command_buffers(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        command_pool: vk::CommandPool,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Vec<vk::CommandBuffer>> {
        debug!(
            "Creating Vulkan graphics command buffers for window with id: {:?}",
            neuclidio_window.id
        );

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(max_frames_in_flight as u32)
            .build();

        let command_buffers = unsafe {
            vulkan_context
                .logical_device
                .allocate_command_buffers(&command_buffer_allocate_info)?
        };

        Ok(command_buffers)
    }

    fn create_shader_module(
        logical_device: &Device,
        bytecode: &[u8],
    ) -> NeuclidioResult<vk::ShaderModule> {
        let bytecode = Bytecode::new(bytecode).map_err(RenderPipelineError::from)?;
        let shader_module_create_info = vk::ShaderModuleCreateInfo::builder()
            .code(bytecode.code())
            .code_size(bytecode.code_size())
            .build();

        let shader_module =
            unsafe { logical_device.create_shader_module(&shader_module_create_info, None)? };

        Ok(shader_module)
    }

    fn destroy_shader_module(logical_device: &Device, shader_module: vk::ShaderModule) {
        unsafe {
            logical_device.destroy_shader_module(shader_module, None);
        }
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
            .primitive_restart_enable(false)
            .build()
    }

    fn create_viewport_state() -> vk::PipelineViewportStateCreateInfo {
        vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1)
            .build()
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
            .build();

        let attachments = vec![attachment];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&attachments)
            .build();

        (color_blend_state, attachments)
    }

    fn create_dynamic_state() -> (vk::PipelineDynamicStateCreateInfo, Vec<vk::DynamicState>) {
        let states = vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&states)
            .build();

        (dynamic_state, states)
    }

    fn create_render_pass(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<vk::RenderPass> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        let color_attachment = vk::AttachmentDescription::builder()
            .format(swap_chain.image_format())
            .samples(vk::SampleCountFlags::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();

        let color_attachment_reference = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let color_attachments = [color_attachment_reference];
        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments)
            .build();

        let offscreen_image_dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .build();

        let attachments = [color_attachment];
        let subpasses = [subpass];
        let dependencies = [offscreen_image_dependency];
        let render_pass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies)
            .build();

        let render_pass = unsafe {
            vulkan_context
                .logical_device
                .create_render_pass(&render_pass_create_info, None)?
        };

        Ok(render_pass)
    }

    fn create_sampler(vulkan_context: &VulkanContext) -> NeuclidioResult<vk::Sampler> {
        let sampler_create_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .anisotropy_enable(false)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0)
            .build();

        let sampler = unsafe {
            vulkan_context
                .logical_device
                .create_sampler(&sampler_create_info, None)?
        };

        Ok(sampler)
    }

    fn create_descriptor_pool(
        vulkan_context: &VulkanContext,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<vk::DescriptorPool> {
        let descriptor_pool_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
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
        allocator_state: &RenderPipelineAllocatorState,
        sampler: vk::Sampler,
        descriptor_set_layout: vk::DescriptorSetLayout,
        descriptor_pool: vk::DescriptorPool,
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

        Self::update_descriptor_sets(
            vulkan_context,
            neuclidio_window,
            allocator_state,
            sampler,
            &descriptor_sets,
            max_frames_in_flight,
        )?;

        Ok(descriptor_sets)
    }

    fn update_descriptor_sets(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        allocator_state: &RenderPipelineAllocatorState,
        sampler: vk::Sampler,
        descriptor_sets: &[vk::DescriptorSet],
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<()> {
        for frame_in_flight_index in 0..max_frames_in_flight {
            let color_image_view =
                allocator_state.color_image_view(neuclidio_window, frame_in_flight_index)?;
            let descriptor_image_info = vk::DescriptorImageInfo::builder()
                .sampler(sampler)
                .image_view(color_image_view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .build();

            let image_info = [descriptor_image_info];
            let write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_sets[frame_in_flight_index])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&image_info)
                .build();

            unsafe {
                vulkan_context.logical_device.update_descriptor_sets(
                    &[write_descriptor_set],
                    &[] as &[vk::CopyDescriptorSet],
                );
            }
        }

        Ok(())
    }
}
