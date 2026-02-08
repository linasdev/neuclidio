use glam::Mat4;
use vulkanalia::vk;
use vulkanalia::vk::HasBuilder;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ViewProjectionUniform {
    view: Mat4,
    projection: Mat4,
}

impl ViewProjectionUniform {
    pub fn new(view: Mat4, projection: Mat4) -> Self {
        Self { view, projection }
    }

    pub fn load_into_uniform_buffer(self, uniform_buffer_memory: *mut u8) {
        let mvp_memory_object = unsafe {
            let mvp_memory: *mut Self = uniform_buffer_memory.cast();
            mvp_memory.as_mut()
        };

        if let Some(mvp_memory_object) = mvp_memory_object {
            *mvp_memory_object = self;
        }
    }

    pub fn descriptor_set_layout_bindings() -> Vec<vk::DescriptorSetLayoutBinding> {
        let descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build();

        vec![descriptor_set_layout_binding]
    }

    pub fn size_in_uniform_buffer() -> vk::DeviceSize {
        size_of::<Self>() as u64
    }
}
