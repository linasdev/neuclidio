use glam::{Vec2, Vec3};
use vulkanalia::vk;
use vulkanalia::vk::HasBuilder;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    position: Vec3,
    normal: Vec3,
    texture_coordinate: Vec2,
}

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3, texture_coordinate: Vec2) -> Self {
        Self {
            position,
            normal,
            texture_coordinate,
        }
    }

    pub fn binding_descriptions() -> Vec<vk::VertexInputBindingDescription> {
        let binding_description = vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build();

        vec![binding_description]
    }

    pub fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        let position_attribute = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        let normal_attribute = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec3>() as u32)
            .build();

        let texture_coordinate_attribute = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(size_of::<Vec3>() as u32 + size_of::<Vec3>() as u32)
            .build();

        vec![
            position_attribute,
            normal_attribute,
            texture_coordinate_attribute,
        ]
    }
}
