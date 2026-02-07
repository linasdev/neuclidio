use glam::Mat4;
use vulkanalia::vk;
use vulkanalia::vk::HasBuilder;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ModelViewProjection {
    model: Mat4,
    view: Mat4,
    projection: Mat4,
}

impl ModelViewProjection {
    pub fn new(model: Mat4, view: Mat4, projection: Mat4) -> Self {
        Self {
            model,
            view,
            projection,
        }
    }

    pub fn descriptor_set_layout_binding() -> Vec<vk::DescriptorSetLayoutBinding> {
        let descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build();

        vec![descriptor_set_layout_binding]
    }

    pub fn size_in_uniform_buffer() -> u32 {
        size_of::<ModelViewProjection>() as u32
    }
}
