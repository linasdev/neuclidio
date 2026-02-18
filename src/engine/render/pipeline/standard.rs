use crate::engine::render::error::RenderError;
use crate::engine::render::pipeline::common::push_constant::PushConstantExt;
use crate::engine::render::pipeline::common::push_constant::model::ModelPushConstant;
use crate::engine::render::pipeline::common::state::allocator::RenderPipelineAllocatorState;
use crate::engine::render::pipeline::common::state::command::RenderPipelineCommandState;
use crate::engine::render::pipeline::common::state::descriptor::RenderPipelineDescriptorState;
use crate::engine::render::pipeline::common::state::display::DisplayPipelineState;
use crate::engine::render::pipeline::common::state::pipeline::RenderPipelineState;
use crate::engine::render::pipeline::common::state::synchronization::RenderPipelineSynchronizationState;
use crate::engine::render::pipeline::common::state::transfer::RenderPipelineTransferState;
use crate::engine::render::pipeline::common::uniform::ViewProjectionUniform;
use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::pipeline::{RenderPipelineExt, get_supported_image_format};
use crate::engine::render::renderable::{Renderable, RenderableExt};
use crate::engine::render::vulkan_context::VulkanContext;
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::entity::Entity;
use crate::entity::transform::{Transform, TransformExt};
use crate::error::NeuclidioResult;
use crate::id::EntityId;
use glam::Mat4;
use log::warn;
use std::collections::HashMap;
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, Handle, HasBuilder, KhrSwapchainExtensionDeviceCommands};
use winit::window::WindowId;

const RENDER_VERTEX_SHADER_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/standard/render_vertex_shader.spv"
));
const RENDER_FRAGMENT_SHADER_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/standard/render_fragment_shader.spv"
));

const DISPLAY_VERTEX_SHADER_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/standard/display_vertex_shader.spv"
));
const DISPLAY_FRAGMENT_SHADER_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/standard/display_fragment_shader.spv"
));

type RenderableEntities = HashMap<Renderable, HashMap<WindowId, HashMap<EntityId, Entity>>>;

pub struct StandardRenderPipeline {
    allocator_state: RenderPipelineAllocatorState,
    descriptor_state: RenderPipelineDescriptorState,
    pipeline_state: RenderPipelineState,
    transfer_state: RenderPipelineTransferState,
    synchronization_state: RenderPipelineSynchronizationState,
    command_state: RenderPipelineCommandState,
    display_state: DisplayPipelineState,
    renderable_entities: RenderableEntities,
}

impl StandardRenderPipeline {}

impl StandardRenderPipeline {
    pub fn new(
        vulkan_context: &VulkanContext,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        if max_frames_in_flight < 2 {
            return Err(RenderPipelineError::MaxFramesInFlightTooLittle.into());
        }

        let color_image_format = get_supported_image_format(
            vulkan_context,
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::COLOR_ATTACHMENT,
            &[vk::Format::R16G16B16A16_SFLOAT],
        )
        .ok_or(RenderError::MissingColorImageFormat)?;

        let depth_stencil_image_format = get_supported_image_format(
            vulkan_context,
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
            &[
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D24_UNORM_S8_UINT,
            ],
        )
        .ok_or(RenderError::MissingDepthStencilImageFormat)?;

        let descriptor_state = RenderPipelineDescriptorState::new(
            vulkan_context,
            &ViewProjectionUniform::descriptor_set_layout_bindings(),
            max_frames_in_flight,
        )?;
        let allocator_state = RenderPipelineAllocatorState::new(
            color_image_format,
            depth_stencil_image_format,
            max_frames_in_flight,
        )?;
        let pipeline_state = RenderPipelineState::new(
            vulkan_context,
            &descriptor_state,
            &allocator_state,
            &[ModelPushConstant::push_constant_range()],
            RENDER_VERTEX_SHADER_BYTECODE,
            RENDER_FRAGMENT_SHADER_BYTECODE,
            max_frames_in_flight,
        )?;
        let transfer_state = RenderPipelineTransferState::new(vulkan_context)?;
        let synchronization_state = RenderPipelineSynchronizationState::new(max_frames_in_flight)?;
        let command_state = RenderPipelineCommandState::new(max_frames_in_flight)?;
        let display_state = DisplayPipelineState::new(vulkan_context, max_frames_in_flight)?;
        let renderable_entities = HashMap::new();

        Ok(Self {
            descriptor_state,
            allocator_state,
            pipeline_state,
            transfer_state,
            synchronization_state,
            command_state,
            display_state,
            renderable_entities,
        })
    }

    fn fill_uniform_buffer(
        neuclidio_window: &NeuclidioWindow,
        uniform_buffer_memory: *mut u8,
    ) -> NeuclidioResult<()> {
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        let view = Mat4::IDENTITY;
        let projection = Mat4::perspective_rh(
            75f32.to_radians(),
            swap_chain.extent().width as f32 / swap_chain.extent().height as f32,
            0.1,
            100.0,
        );

        ViewProjectionUniform::new(view, projection)
            .load_into_uniform_buffer(uniform_buffer_memory);

        Ok(())
    }

    fn record_command_buffer(
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        pipeline_state: &RenderPipelineState,
        descriptor_state: &RenderPipelineDescriptorState,
        allocator_state: &RenderPipelineAllocatorState,
        synchronization_state: &RenderPipelineSynchronizationState,
        renderable_entities: &RenderableEntities,
        command_buffer: vk::CommandBuffer,
        frame_in_flight_index: usize,
    ) -> NeuclidioResult<()> {
        let frame_index = synchronization_state.frame_index(neuclidio_window)?;
        let descriptor_set =
            descriptor_state.descriptor_set(neuclidio_window, frame_in_flight_index)?;

        let logical_device = &vulkan_context.logical_device;
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        let frame_buffer = pipeline_state
            .frame_buffer(neuclidio_window, frame_in_flight_index)
            .ok_or(RenderPipelineError::Unprepared)?;

        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(swap_chain.extent())
            .build();

        let clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let depth_clear_value = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };

        let clear_values = [clear_value, depth_clear_value];
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(pipeline_state.render_pass())
            .framebuffer(frame_buffer)
            .render_area(render_area)
            .clear_values(&clear_values)
            .build();

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(swap_chain.extent().width as f32)
            .height(swap_chain.extent().height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build();
        let viewports = [viewport];

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(swap_chain.extent())
            .build();
        let scissors = [scissor];

        unsafe {
            logical_device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            logical_device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_state.pipeline(),
            );

            logical_device.cmd_set_viewport(command_buffer, 0, &viewports);
            logical_device.cmd_set_scissor(command_buffer, 0, &scissors);
        }

        for (renderable, entities_by_window) in renderable_entities.iter() {
            let entities = if let Some(entities) = entities_by_window.get(&neuclidio_window.id) {
                entities
            } else {
                continue;
            };

            let (render_buffer_id, render_buffer_offset, last_used_in_frame) = if let (
                Some(render_buffer_id),
                Some(render_buffer_offset),
                Some(last_used_in_frame),
            ) = (
                renderable.render_buffer_id(),
                renderable.render_buffer_offset(),
                renderable.last_used_in_frame(),
            ) {
                (render_buffer_id, render_buffer_offset, last_used_in_frame)
            } else {
                warn!(
                    "Tried to render renderable with id '{:?}' without it being in the render buffer for window with id: {:?}",
                    renderable.id(),
                    neuclidio_window.id
                );
                continue;
            };

            let render_buffer = if let Some(render_buffer) =
                allocator_state.render_buffer(render_buffer_id)
            {
                render_buffer
            } else {
                warn!(
                    "Could not find render buffer with index '{render_buffer_id}' for window with id: {:?}",
                    neuclidio_window.id
                );
                continue;
            };

            unsafe {
                logical_device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[render_buffer],
                    &[render_buffer_offset],
                );
                logical_device.cmd_bind_index_buffer(
                    command_buffer,
                    render_buffer,
                    render_buffer_offset + renderable.index_offset(),
                    vk::IndexType::UINT32,
                );
                logical_device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline_state.pipeline_layout(),
                    0,
                    &[descriptor_set],
                    &[],
                );
            }

            for (_, entity) in entities.iter() {
                unsafe {
                    entity.do_with_transform::<Transform, _>(|transform| {
                        let push_constant = transform.as_push_constant();
                        logical_device.cmd_push_constants(
                            command_buffer,
                            pipeline_state.pipeline_layout(),
                            vk::ShaderStageFlags::VERTEX,
                            0,
                            push_constant.as_bytes(),
                        );
                    });

                    logical_device.cmd_draw_indexed(
                        command_buffer,
                        renderable.index_count() as u32,
                        1,
                        0,
                        0,
                        0,
                    );
                }
            }

            last_used_in_frame
                .lock()
                .unwrap()
                .insert(neuclidio_window.id, frame_index + 1);
        }

        unsafe {
            logical_device.cmd_end_render_pass(command_buffer);
        }

        Ok(())
    }
}

impl RenderPipelineExt for StandardRenderPipeline {
    fn submit_entity(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
        entity: Entity,
    ) -> NeuclidioResult<()> {
        let mut new_renderables = vec![];
        let renderables = entity.get_renderables();
        for renderable in renderables.into_iter() {
            if let Some(entities_by_window) = self.renderable_entities.get_mut(&renderable) {
                if let Some(entities) = entities_by_window.get_mut(&neuclidio_window.id) {
                    entities.insert(entity.id(), entity.clone());
                } else {
                    let mut entities = HashMap::new();
                    entities.insert(entity.id(), entity.clone());
                    entities_by_window.insert(neuclidio_window.id, entities);
                }
            } else {
                let mut entities = HashMap::new();
                entities.insert(entity.id(), entity.clone());

                let mut entities_by_window = HashMap::new();
                entities_by_window.insert(neuclidio_window.id, entities);

                self.renderable_entities
                    .insert(renderable.clone(), entities_by_window);
                new_renderables.push(renderable);
            }
        }

        if !new_renderables.is_empty() {
            self.allocator_state.submit_renderables(
                vulkan_context,
                &self.transfer_state,
                &new_renderables,
            )?;
        }

        Ok(())
    }

    fn remove_entity(&mut self, window_ids: Vec<WindowId>, entity: Entity) -> NeuclidioResult<()> {
        let mut removed_renderables = vec![];
        let renderables = entity.get_renderables();
        for renderable in renderables.into_iter() {
            if let Some(entities_by_window) = self.renderable_entities.get_mut(&renderable) {
                for window_id in window_ids.iter() {
                    if let Some(entities) = entities_by_window.get_mut(&window_id) {
                        entities.remove(&entity.id());
                    }

                    if let Some(entities) = entities_by_window.get(&window_id)
                        && entities.is_empty()
                    {
                        entities_by_window.remove(&window_id);
                    }
                }
            }

            if let Some(entities_by_window) = self.renderable_entities.get(&renderable)
                && entities_by_window.is_empty()
            {
                self.renderable_entities.remove(&renderable);
                removed_renderables.push(renderable);
            }
        }

        if !removed_renderables.is_empty() {
            self.allocator_state
                .remove_renderables(&removed_renderables)?;
        }

        Ok(())
    }

    fn handle_renderable_added(
        &mut self,
        vulkan_context: &VulkanContext,
        entity: Entity,
        renderable: Renderable,
    ) -> NeuclidioResult<()> {
        if let Some(entities_by_window) = self.renderable_entities.get_mut(&renderable) {
            entity.do_with_each_window_id(|window_id| {
                if let Some(entities) = entities_by_window.get_mut(&window_id) {
                    entities.insert(entity.id(), entity.clone());
                } else {
                    let mut entities = HashMap::new();
                    entities.insert(entity.id(), entity.clone());
                    entities_by_window.insert(window_id, entities);
                }
            });
        } else {
            let mut entities = HashMap::new();
            entities.insert(entity.id(), entity.clone());

            let mut entities_by_window = HashMap::new();
            entity.do_with_each_window_id(|window_id| {
                entities_by_window.insert(window_id, entities.clone());
            });

            self.renderable_entities
                .insert(renderable.clone(), entities_by_window);

            self.allocator_state.submit_renderables(
                vulkan_context,
                &self.transfer_state,
                &[renderable],
            )?;
        }

        Ok(())
    }

    fn handle_renderable_removed(
        &mut self,
        entity: Entity,
        renderable: Renderable,
    ) -> NeuclidioResult<()> {
        if let Some(entities_by_window) = self.renderable_entities.get_mut(&renderable) {
            entity.do_with_each_window_id(|window_id| {
                if let Some(entities) = entities_by_window.get_mut(&window_id) {
                    entities.remove(&entity.id());
                }

                if let Some(entities) = entities_by_window.get(&window_id)
                    && entities.is_empty()
                {
                    entities_by_window.remove(&window_id);
                }
            });
        }

        if let Some(entities) = self.renderable_entities.get(&renderable)
            && entities.is_empty()
        {
            self.renderable_entities.remove(&renderable);
            self.allocator_state.remove_renderables(&[renderable])?;
        }

        Ok(())
    }

    fn render(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_windows: &HashMap<WindowId, NeuclidioWindow>,
    ) -> NeuclidioResult<Vec<WindowId>> {
        // TODO: Move this out of the main loop and do often but not every frame.
        self.allocator_state
            .deallocate_renderables(vulkan_context, &self.synchronization_state)?;

        let mut changed_window_ids = vec![];

        for neuclidio_window in neuclidio_windows.values() {
            let logical_device = &vulkan_context.logical_device;
            let swap_chain = neuclidio_window
                .swap_chain
                .as_ref()
                .ok_or(RenderPipelineError::Unprepared)?;

            // Wait for GPU

            self.synchronization_state
                .wait_for_frame_index_semaphore_value(vulkan_context, neuclidio_window)?;

            // Acquire indices

            let frame_in_flight_index = self
                .synchronization_state
                .frame_in_flight_index(neuclidio_window)?;

            let current_image_available_semaphore = self
                .synchronization_state
                .current_image_available_semaphore(neuclidio_window)?;

            let image_index_result = unsafe {
                logical_device.acquire_next_image_khr(
                    swap_chain.chain(),
                    u64::MAX,
                    current_image_available_semaphore,
                    vk::Fence::null(),
                )
            };

            let image_index = match image_index_result {
                Ok((image_index, _)) => image_index as usize,
                Err(vk::ErrorCode::OUT_OF_DATE_KHR) => {
                    changed_window_ids.push(neuclidio_window.id);
                    continue;
                }
                Err(err) => return Err(err.into()),
            };

            // Render pass

            self.command_state.record_command_buffer(
                vulkan_context,
                neuclidio_window,
                frame_in_flight_index,
                |command_buffer| {
                    Self::record_command_buffer(
                        vulkan_context,
                        neuclidio_window,
                        &self.pipeline_state,
                        &self.descriptor_state,
                        &self.allocator_state,
                        &self.synchronization_state,
                        &self.renderable_entities,
                        command_buffer,
                        frame_in_flight_index,
                    )
                },
            )?;
            self.allocator_state.fill_uniform_buffer(
                vulkan_context,
                neuclidio_window,
                frame_in_flight_index,
                |uniform_buffer_memory| {
                    Self::fill_uniform_buffer(&neuclidio_window, uniform_buffer_memory)
                },
            )?;

            let frame_index_semaphore_required_value = self
                .synchronization_state
                .frame_index_semaphore_required_value(neuclidio_window)?;
            let frame_index_semaphore_first_value = self
                .synchronization_state
                .frame_index_semaphore_first_value(neuclidio_window)?;

            let wait_semaphore_values = [frame_index_semaphore_required_value];
            let signal_semaphore_values = [frame_index_semaphore_first_value];
            let mut timeline_semaphore_submit_info = vk::TimelineSemaphoreSubmitInfo::builder()
                .wait_semaphore_values(&wait_semaphore_values)
                .signal_semaphore_values(&signal_semaphore_values)
                .build();

            let frame_index_semaphore = self
                .synchronization_state
                .frame_index_semaphore(neuclidio_window)?;

            let command_buffer = self
                .command_state
                .command_buffer(neuclidio_window, frame_in_flight_index)?;

            let wait_semaphores = [frame_index_semaphore];
            let wait_dst_stage_mask = [vk::PipelineStageFlags::TOP_OF_PIPE];
            let command_buffers = [command_buffer];
            let signal_semaphores = [frame_index_semaphore];
            let submit_info = vk::SubmitInfo::builder()
                .push_next(&mut timeline_semaphore_submit_info)
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_dst_stage_mask)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores)
                .build();

            unsafe {
                logical_device.queue_submit(
                    vulkan_context.graphics_queue,
                    &[submit_info],
                    vk::Fence::null(),
                )?;
            }

            // Display pass

            self.display_state.record_command_buffer(
                vulkan_context,
                neuclidio_window,
                frame_in_flight_index,
                image_index,
            )?;

            let frame_index_semaphore_second_value = self
                .synchronization_state
                .frame_index_semaphore_second_value(neuclidio_window)?;

            let wait_semaphore_values = [0, frame_index_semaphore_first_value];
            let signal_semaphore_values = [0, frame_index_semaphore_second_value];
            let mut timeline_semaphore_submit_info = vk::TimelineSemaphoreSubmitInfo::builder()
                .wait_semaphore_values(&wait_semaphore_values)
                .signal_semaphore_values(&signal_semaphore_values)
                .build();

            let current_render_finished_semaphore = self
                .synchronization_state
                .current_render_finished_semaphore(neuclidio_window, image_index)?;

            let command_buffer = self
                .display_state
                .command_buffer(neuclidio_window, frame_in_flight_index)?;

            let wait_semaphores = [current_image_available_semaphore, frame_index_semaphore];
            let wait_dst_stage_mask = [
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            ];
            let command_buffers = [command_buffer];
            let signal_semaphores = [current_render_finished_semaphore, frame_index_semaphore];
            let submit_info = vk::SubmitInfo::builder()
                .push_next(&mut timeline_semaphore_submit_info)
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_dst_stage_mask)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores)
                .build();

            unsafe {
                logical_device.queue_submit(
                    vulkan_context.graphics_queue,
                    &[submit_info],
                    vk::Fence::null(),
                )?;
            }

            // Presentation

            let wait_semaphores = [current_render_finished_semaphore];
            let swap_chains = [swap_chain.chain()];
            let image_indices = [image_index as u32];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&wait_semaphores)
                .swapchains(&swap_chains)
                .image_indices(&image_indices)
                .build();

            let present_result = unsafe {
                logical_device.queue_present_khr(vulkan_context.present_queue, &present_info)
            };

            match present_result {
                Ok(_) => {}
                Err(vk::ErrorCode::OUT_OF_DATE_KHR) => {
                    changed_window_ids.push(neuclidio_window.id);
                    continue;
                }
                Err(err) => return Err(err.into()),
            };

            self.synchronization_state
                .increment_frame(neuclidio_window)?;
        }

        Ok(changed_window_ids)
    }

    fn prepare_for_window_reset(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) {
        self.display_state
            .prepare_for_window_reset(vulkan_context, neuclidio_window);
        self.synchronization_state
            .prepare_for_window_reset(vulkan_context, neuclidio_window);
        self.pipeline_state
            .prepare_for_window_reset(vulkan_context, neuclidio_window);
        self.descriptor_state
            .prepare_for_window_reset(vulkan_context, neuclidio_window);
        self.allocator_state
            .prepare_for_window_reset(vulkan_context, neuclidio_window);
    }

    fn reset_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) -> NeuclidioResult<()> {
        self.allocator_state.reset_window(
            vulkan_context,
            neuclidio_window,
            ViewProjectionUniform::size_in_uniform_buffer(),
        )?;
        self.descriptor_state.reset_window(
            vulkan_context,
            neuclidio_window,
            &self.allocator_state,
            ViewProjectionUniform::size_in_uniform_buffer(),
        )?;
        self.pipeline_state.reset_window(
            vulkan_context,
            neuclidio_window,
            &self.allocator_state,
        )?;
        self.synchronization_state
            .reset_window(vulkan_context, neuclidio_window)?;
        self.command_state
            .reset_window(vulkan_context, neuclidio_window)?;
        self.display_state.reset_window(
            vulkan_context,
            neuclidio_window,
            &self.allocator_state,
            DISPLAY_VERTEX_SHADER_BYTECODE,
            DISPLAY_FRAGMENT_SHADER_BYTECODE,
        )?;

        Ok(())
    }

    fn clean_up_for_window(
        &mut self,
        vulkan_context: &VulkanContext,
        neuclidio_window: &NeuclidioWindow,
    ) {
        self.allocator_state
            .clean_up_for_window(vulkan_context, neuclidio_window);
        self.descriptor_state
            .clean_up_for_window(vulkan_context, neuclidio_window);
        self.pipeline_state
            .clean_up_for_window(vulkan_context, neuclidio_window);
        self.synchronization_state
            .clean_up_for_window(vulkan_context, neuclidio_window);
        self.command_state
            .clean_up_for_window(vulkan_context, neuclidio_window);
        self.display_state
            .clean_up_for_window(vulkan_context, neuclidio_window);
    }

    fn destroy(self, vulkan_context: &VulkanContext) {
        self.display_state.destroy(vulkan_context);
        self.command_state.destroy(vulkan_context);
        self.synchronization_state.destroy(vulkan_context);
        self.transfer_state.destroy(vulkan_context);
        self.pipeline_state.destroy(vulkan_context);
        self.descriptor_state.destroy(vulkan_context);
        self.allocator_state.destroy(vulkan_context);
    }
}
