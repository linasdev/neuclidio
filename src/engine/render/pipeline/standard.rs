use crate::component::mesh::loader::MeshLoader;
use crate::engine::render::error::RenderError;
use crate::engine::render::pipeline::RenderPipelineExt;
use crate::engine::render::pipeline::common::push_constant::PushConstantExt;
use crate::engine::render::pipeline::common::push_constant::model::ModelPushConstant;
use crate::engine::render::pipeline::common::state::allocator::RenderPipelineAllocatorState;
use crate::engine::render::pipeline::common::state::command::RenderPipelineCommandState;
use crate::engine::render::pipeline::common::state::descriptor::RenderPipelineDescriptorState;
use crate::engine::render::pipeline::common::state::pipeline::RenderPipelineState;
use crate::engine::render::pipeline::common::state::synchronization::RenderPipelineSynchronizationState;
use crate::engine::render::pipeline::common::uniform::ViewProjectionUniform;
use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::renderable::{Renderable, RenderableExt};
use crate::engine::render::windowing::window::NeuclidioWindow;
use crate::entity::transform::euclidean::EuclideanTransform;
use crate::entity::transform::{Transform, TransformExt};
use crate::entity::{Entity, EntityId};
use crate::error::NeuclidioResult;
use glam::{Mat4, Quat, Vec3};
use std::collections::BTreeMap;
use std::time::Instant;
use vulkanalia::vk;
use vulkanalia::vk::{DeviceV1_0, Handle, HasBuilder, KhrSwapchainExtensionDeviceCommands};

const VERTEX_SHADER_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/standard/vertex_shader.spv"
));
const FRAGMENT_SHADER_BYTECODE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/shaders/standard/fragment_shader.spv"
));

type RenderableEntities = BTreeMap<Renderable, BTreeMap<EntityId, Entity>>;

pub struct StandardRenderPipeline {
    pipeline_state: Option<RenderPipelineState>,
    descriptor_state: RenderPipelineDescriptorState,
    synchronization_state: RenderPipelineSynchronizationState,
    command_state: RenderPipelineCommandState,
    allocator_state: RenderPipelineAllocatorState,
    renderable_entities: RenderableEntities,
}

impl StandardRenderPipeline {}

impl StandardRenderPipeline {
    pub fn new(
        neuclidio_window: &NeuclidioWindow,
        max_frames_in_flight: usize,
    ) -> NeuclidioResult<Self> {
        let descriptor_state = RenderPipelineDescriptorState::new(
            neuclidio_window,
            &ViewProjectionUniform::descriptor_set_layout_bindings(),
        )?;
        let synchronization_state =
            RenderPipelineSynchronizationState::new(neuclidio_window, max_frames_in_flight)?;
        let command_state = RenderPipelineCommandState::new(neuclidio_window)?;
        let allocator_state = RenderPipelineAllocatorState::new(neuclidio_window)?;
        let renderable_entities = BTreeMap::new();

        Ok(Self {
            pipeline_state: None,
            descriptor_state,
            synchronization_state,
            command_state,
            allocator_state,
            renderable_entities,
        })
    }

    pub fn submit_entity(
        &mut self,
        neuclidio_window: &NeuclidioWindow,
        entity: &Entity,
    ) -> NeuclidioResult<()> {
        let mut renderables_changed = false;
        let renderables = entity.get_renderables();
        for renderable in renderables.into_iter() {
            if let Some(entities) = self.renderable_entities.get_mut(&renderable) {
                entities.insert(entity.id(), entity.clone());
            } else {
                let mut entities = BTreeMap::new();
                entities.insert(entity.id(), entity.clone());
                self.renderable_entities.insert(renderable, entities);
                renderables_changed = true;
            }
        }

        if renderables_changed {
            self.allocator_state.fill_render_buffer(
                neuclidio_window,
                &self.command_state,
                self.render_buffer_size(),
                |render_buffer_memory| {
                    Self::fill_render_buffer(&self.renderable_entities, render_buffer_memory);
                },
            )?;
        }

        Ok(())
    }

    pub fn remove_entity(
        &mut self,
        neuclidio_window: &NeuclidioWindow,
        entity: &Entity,
    ) -> NeuclidioResult<()> {
        let mut renderables_changed = false;
        let renderables = entity.get_renderables();
        for renderable in renderables.into_iter() {
            if let Some(entities) = self.renderable_entities.get_mut(&renderable) {
                entities.remove(&entity.id());
            }

            if let Some(entities) = self.renderable_entities.get(&renderable)
                && entities.is_empty()
            {
                self.renderable_entities.remove(&renderable);
                renderables_changed = true;
            }
        }

        if renderables_changed {
            self.allocator_state.fill_render_buffer(
                neuclidio_window,
                &self.command_state,
                self.render_buffer_size(),
                |render_buffer_memory| {
                    Self::fill_render_buffer(&self.renderable_entities, render_buffer_memory);
                },
            )?;
        }

        Ok(())
    }

    fn render_buffer_size(&self) -> vk::DeviceSize {
        self.renderable_entities
            .keys()
            .map(|renderable| renderable.size_in_render_buffer())
            .sum()
    }

    fn fill_render_buffer(
        renderable_entities: &RenderableEntities,
        mut render_buffer_memory: *mut u8,
    ) {
        for renderable in renderable_entities.keys() {
            renderable.load_into_render_buffer(render_buffer_memory);
            render_buffer_memory =
                unsafe { render_buffer_memory.add(renderable.size_in_render_buffer() as usize) };
        }
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
        neuclidio_window: &NeuclidioWindow,
        pipeline_state: &Option<RenderPipelineState>,
        descriptor_state: &RenderPipelineDescriptorState,
        allocator_state: &RenderPipelineAllocatorState,
        renderable_entities: &RenderableEntities,
        command_buffer_index: usize,
        command_buffer: vk::CommandBuffer,
    ) -> NeuclidioResult<()> {
        let descriptor_set = descriptor_state.descriptor_set(command_buffer_index)?;
        let pipeline_state = pipeline_state
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        let logical_device = &neuclidio_window.logical_device;
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
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

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(pipeline_state.render_pass())
            .framebuffer(pipeline_state.frame_buffer(command_buffer_index))
            .render_area(render_area)
            .clear_values(&[clear_value, depth_clear_value])
            .build();

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
        }

        if let Some(render_buffer) = allocator_state.render_buffer() {
            let mut current_render_buffer_offset = 0;
            for (renderable, entities_with_renderable) in renderable_entities.iter() {
                unsafe {
                    logical_device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[render_buffer],
                        &[current_render_buffer_offset],
                    );
                    logical_device.cmd_bind_index_buffer(
                        command_buffer,
                        render_buffer,
                        current_render_buffer_offset + renderable.index_offset(),
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

                for (_, entity) in entities_with_renderable.iter() {
                    unsafe {
                        entity.do_with_transform(|transform| {
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

                current_render_buffer_offset += renderable.size_in_render_buffer();
            }
        }

        unsafe {
            logical_device.cmd_end_render_pass(command_buffer);
        }

        Ok(())
    }
}

impl RenderPipelineExt for StandardRenderPipeline {
    fn render(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()> {
        let logical_device = &neuclidio_window.logical_device;
        let swap_chain = neuclidio_window
            .swap_chain
            .as_ref()
            .ok_or(RenderPipelineError::Unprepared)?;

        self.synchronization_state
            .wait_for_in_flight_fence(neuclidio_window)?;

        let image_index_result = unsafe {
            logical_device.acquire_next_image_khr(
                swap_chain.chain(),
                u64::MAX,
                self.synchronization_state
                    .get_current_image_available_semaphore(),
                vk::Fence::null(),
            )
        };

        let image_index = match image_index_result {
            Ok((image_index, _)) => image_index as usize,
            Err(vk::ErrorCode::OUT_OF_DATE_KHR) => {
                return Err(RenderError::OutOfDateSwapChain.into());
            }
            Err(err) => return Err(err.into()),
        };

        self.synchronization_state
            .wait_for_image_in_flight(logical_device, image_index)?;
        self.synchronization_state
            .set_image_in_flight_to_current_in_flight_fence(image_index)?;

        self.command_state.record_command_buffer(
            neuclidio_window,
            image_index,
            |command_buffer| {
                Self::record_command_buffer(
                    neuclidio_window,
                    &self.pipeline_state,
                    &self.descriptor_state,
                    &self.allocator_state,
                    &self.renderable_entities,
                    image_index,
                    command_buffer,
                )
            },
        )?;
        self.allocator_state
            .fill_uniform_buffer(image_index, |uniform_buffer_memory| {
                Self::fill_uniform_buffer(&neuclidio_window, uniform_buffer_memory)
            })?;

        self.synchronization_state
            .reset_current_in_flight_fence(neuclidio_window)?;

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&[self
                .synchronization_state
                .get_current_image_available_semaphore()])
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&[self.command_state.command_buffer(image_index)?])
            .signal_semaphores(&[self
                .synchronization_state
                .get_current_render_finished_semaphore()])
            .build();

        unsafe {
            logical_device.queue_submit(
                neuclidio_window.graphics_queue,
                &[submit_info],
                self.synchronization_state.get_current_in_flight_fence(),
            )?;
        }

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&[self
                .synchronization_state
                .get_current_render_finished_semaphore()])
            .swapchains(&[swap_chain.chain()])
            .image_indices(&[image_index as u32])
            .build();

        unsafe {
            logical_device.queue_present_khr(neuclidio_window.present_queue, &present_info)?;
        }

        self.synchronization_state.increment_frame();

        Ok(())
    }

    fn prepare_for_reset(&mut self, neuclidio_window: &NeuclidioWindow) {
        self.command_state.prepare_for_reset(neuclidio_window);

        if let Some(pipeline_state) = self.pipeline_state.take() {
            pipeline_state.destroy(neuclidio_window);
        }

        self.descriptor_state.prepare_for_reset(neuclidio_window);
        self.allocator_state.prepare_for_reset(neuclidio_window);
        self.synchronization_state.prepare_for_reset();
    }

    fn reset(&mut self, neuclidio_window: &NeuclidioWindow) -> NeuclidioResult<()> {
        self.synchronization_state.reset(neuclidio_window)?;
        self.allocator_state.reset(
            neuclidio_window,
            ViewProjectionUniform::size_in_uniform_buffer(),
        )?;
        self.descriptor_state.reset(
            neuclidio_window,
            &self.allocator_state,
            ViewProjectionUniform::size_in_uniform_buffer(),
        )?;

        self.pipeline_state = Some(RenderPipelineState::new(
            neuclidio_window,
            &self.descriptor_state,
            &self.allocator_state,
            &[ModelPushConstant::push_constant_range()],
            VERTEX_SHADER_BYTECODE,
            FRAGMENT_SHADER_BYTECODE,
        )?);

        self.command_state.reset(neuclidio_window)?;

        Ok(())
    }

    fn destroy(mut self, neuclidio_window: &NeuclidioWindow) {
        self.command_state.destroy(neuclidio_window);

        if let Some(pipeline_state) = self.pipeline_state.take() {
            pipeline_state.destroy(neuclidio_window);
        }

        self.descriptor_state.destroy(neuclidio_window);
        self.allocator_state.destroy(neuclidio_window);
        self.synchronization_state.destroy(neuclidio_window);
    }
}
