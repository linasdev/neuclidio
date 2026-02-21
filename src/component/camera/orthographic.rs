use crate::component::ComponentExt;
use crate::component::camera::CameraExt;
use crate::id::ComponentId;
use glam::Mat4;

#[derive(Clone)]
pub struct OrthographicCamera {
    id: ComponentId,
    size: OrthographicCameraSize,
    z_near_clipping_distance: f32,
    z_far_clipping_distance: f32,
    projection_matrix: Option<Mat4>,
}

impl OrthographicCamera {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> OrthographicCameraBuilder {
        OrthographicCameraBuilder::default()
    }
}

impl CameraExt for OrthographicCamera {
    fn get_view_matrix(&self) -> Mat4 {
        todo!()
    }

    fn get_projection_matrix(&mut self, aspect_ratio: f32) -> Mat4 {
        if let Some(projection_matrix) = self.projection_matrix {
            projection_matrix
        } else {
            let (half_width, half_height) = match self.size {
                OrthographicCameraSize::VerticallyConstrained(height) => {
                    let half_height = height / 2.0;
                    let half_width = aspect_ratio * half_height;
                    (half_width, half_height)
                }
                OrthographicCameraSize::HorizontallyConstrained(width) => {
                    let half_width = width / 2.0;
                    let half_height = half_width / aspect_ratio;
                    (half_width, half_height)
                }
            };

            let projection_matrix = Mat4::orthographic_rh(
                -half_width,
                half_width,
                -half_height,
                half_height,
                self.z_near_clipping_distance,
                self.z_far_clipping_distance,
            );

            self.projection_matrix = Some(projection_matrix);

            projection_matrix
        }
    }
}

impl ComponentExt for OrthographicCamera {
    fn id(&self) -> ComponentId {
        self.id
    }
}

impl Default for OrthographicCamera {
    fn default() -> Self {
        OrthographicCamera::builder().build()
    }
}

#[derive(Copy, Clone)]
pub enum OrthographicCameraSize {
    VerticallyConstrained(f32),
    HorizontallyConstrained(f32),
}

pub struct OrthographicCameraBuilder {
    size: OrthographicCameraSize,
    z_near_clipping_distance: f32,
    z_far_clipping_distance: f32,
}

impl OrthographicCameraBuilder {
    pub fn with_size(mut self, size: OrthographicCameraSize) -> Self {
        self.size = size;
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.size = OrthographicCameraSize::VerticallyConstrained(height);
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.size = OrthographicCameraSize::HorizontallyConstrained(width);
        self
    }

    pub fn with_z_near_clipping_distance(mut self, z_near_clipping_distance: f32) -> Self {
        self.z_near_clipping_distance = z_near_clipping_distance;
        self
    }

    pub fn with_z_far_clipping_distance(mut self, z_far_clipping_distance: f32) -> Self {
        self.z_far_clipping_distance = z_far_clipping_distance;
        self
    }

    pub fn build(self) -> OrthographicCamera {
        OrthographicCamera {
            id: ComponentId::new(),
            size: self.size,
            z_near_clipping_distance: self.z_near_clipping_distance,
            z_far_clipping_distance: self.z_far_clipping_distance,
            projection_matrix: None,
        }
    }
}

impl Default for OrthographicCameraBuilder {
    fn default() -> Self {
        Self {
            size: OrthographicCameraSize::VerticallyConstrained(5.0),
            z_near_clipping_distance: 0.1,
            z_far_clipping_distance: 1000.0,
        }
    }
}
