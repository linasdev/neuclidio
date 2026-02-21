use crate::id::RenderBufferId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use vulkanalia::vk;
use vulkanalia_vma::VirtualAllocation;
use winit::window::WindowId;

pub(crate) struct RenderableMemoryAllocation {
    pub render_buffer_id: RenderBufferId,
    pub virtual_allocation: VirtualAllocation,
    pub offset: vk::DeviceSize,
    pub last_used_in_frame: Arc<Mutex<HashMap<WindowId, u64>>>,
}
