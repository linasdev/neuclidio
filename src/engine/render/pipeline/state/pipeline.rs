use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::pipeline::state::descriptor::RenderPipelineDescriptorState;
use crate::engine::render::pipeline::vertex::Vertex;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::error::NeuclidioResult;
use log::debug;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::vk::{DeviceV1_0, Handle, HasBuilder};
use vulkanalia::{Device, vk};

pub struct RenderPipelineState {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    frame_buffers: Vec<vk::Framebuffer>,
}

impl RenderPipelineState {
    pub fn new(
        neuclidio_window: &NeuclidioWindow,
        descriptor_state: &RenderPipelineDescriptorState,
        vertex_shader_bytecode: &[u8],
        fragment_shader_bytecode: &[u8],
    ) -> NeuclidioResult<Self> {
        let logical_device = &neuclidio_window.logical_device;

        let vertex_shader_module =
            Self::create_shader_module(logical_device, vertex_shader_bytecode)?;
        let fragment_shader_module =
            Self::create_shader_module(logical_device, fragment_shader_bytecode)?;

        let vertex_shader_stage = Self::create_vertex_shader_stage(vertex_shader_module);
        let fragment_shader_stage = Self::create_fragment_shader_stage(fragment_shader_module);
        let vertex_input_state = Self::create_vertex_input_state();
        let input_assembly_state = Self::create_input_assembly_state();
        let viewport_state = Self::create_viewport_state(neuclidio_window)?;
        let rasterization_state = Self::create_rasterization_state();
        let multisample_state = Self::create_multisample_state();
        let color_blend_state = Self::create_color_blend_state();

        debug!(
            "Creating Vulkan pipeline layout for window with id: {:?}",
            neuclidio_window.id
        );

        let pipeline_layout = Self::create_pipeline_layout(
            neuclidio_window,
            descriptor_state.descriptor_set_layout(),
        )?;

        debug!(
            "Creating Vulkan render pass for window with id: {:?}",
            neuclidio_window.id
        );

        let render_pass = Self::create_render_pass(neuclidio_window)?;

        debug!(
            "Creating Vulkan pipeline for window with id: {:?}",
            neuclidio_window.id
        );

        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&[vertex_shader_stage, fragment_shader_stage])
            .vertex_input_state(&vertex_input_state.0)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state.0)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state.0)
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
            neuclidio_window.id
        );

        let frame_buffers = Self::create_frame_buffers(neuclidio_window, render_pass)?;

        Ok(Self {
            pipeline,
            pipeline_layout,
            render_pass,
            frame_buffers,
        })
    }

    pub fn destroy(self, neuclidio_window: &NeuclidioWindow) {
        let logical_device = &neuclidio_window.logical_device;

        debug!(
            "Destroying Vulkan frame buffers for window with id: {:?}",
            neuclidio_window.id
        );

        for frame_buffer in self.frame_buffers.iter() {
            unsafe {
                logical_device.destroy_framebuffer(*frame_buffer, None);
            }
        }

        debug!(
            "Destroying Vulkan pipeline for window with id: {:?}",
            neuclidio_window.id
        );

        unsafe {
            logical_device.destroy_pipeline(self.pipeline, None);
        }

        debug!(
            "Destroying Vulkan pipeline layout for window with id: {:?}",
            neuclidio_window.id
        );

        unsafe {
            logical_device.destroy_pipeline_layout(self.pipeline_layout, None);
        }

        debug!(
            "Destroying Vulkan render pass for window with id: {:?}",
            neuclidio_window.id
        );

        unsafe {
            logical_device.destroy_render_pass(self.render_pass, None);
        }
    }

    pub fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }

    pub fn pipeline_layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }

    pub fn render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }

    pub fn frame_buffer(&self, frame_buffer_index: usize) -> vk::Framebuffer {
        self.frame_buffers[frame_buffer_index]
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

    fn create_vertex_input_state() -> (
        vk::PipelineVertexInputStateCreateInfo,
        Vec<vk::VertexInputBindingDescription>,
        Vec<vk::VertexInputAttributeDescription>,
    ) {
        let binding_descriptions = Vertex::binding_descriptions();
        let attribute_descriptions = Vertex::attribute_descriptions();
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions)
            .build();

        (
            vertex_input_state,
            binding_descriptions,
            attribute_descriptions,
        )
    }

    fn create_input_assembly_state() -> vk::PipelineInputAssemblyStateCreateInfo {
        vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .build()
    }

    fn create_viewport_state(
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<(
        vk::PipelineViewportStateCreateInfo,
        Vec<vk::Viewport>,
        Vec<vk::Rect2D>,
    )> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(swap_chain.extent().width as f32)
            .height(swap_chain.extent().height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build();

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(swap_chain.extent())
            .build();

        let viewports = vec![viewport];
        let scissors = vec![scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors)
            .build();

        Ok((viewport_state, viewports, scissors))
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

    fn create_pipeline_layout(
        neuclidio_window: &NeuclidioWindow,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> NeuclidioResult<vk::PipelineLayout> {
        let set_layouts = vec![descriptor_set_layout];
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&set_layouts)
            .build();

        let pipeline_layout = unsafe {
            neuclidio_window
                .logical_device
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };

        Ok(pipeline_layout)
    }

    fn create_render_pass(neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<vk::RenderPass> {
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

        let render_pass = unsafe {
            neuclidio_window
                .logical_device
                .create_render_pass(&render_pass_create_info, None)?
        };

        Ok(render_pass)
    }

    fn create_frame_buffers(
        neuclidio_window: &NeuclidioWindow,
        render_pass: vk::RenderPass,
    ) -> NeuclidioResult<Vec<vk::Framebuffer>> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        let mut frame_buffers = Vec::with_capacity(swap_chain.image_count());

        for image_view in swap_chain.image_views() {
            let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&[*image_view])
                .width(swap_chain.extent().width)
                .height(swap_chain.extent().height)
                .layers(1)
                .build();

            let frame_buffer = unsafe {
                neuclidio_window
                    .logical_device
                    .create_framebuffer(&frame_buffer_create_info, None)?
            };

            frame_buffers.push(frame_buffer);
        }

        Ok(frame_buffers)
    }
}
