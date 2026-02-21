use crate::component::ComponentExt;
use crate::component::camera::CameraExt;
use crate::id::ComponentId;
use glam::Mat4;

#[derive(Clone)]
pub struct PerspectiveCamera {
    id: ComponentId,
    field_of_view_radians: f32,
    z_near_clipping_distance: f32,
    z_far_clipping_distance: f32,
    projection_matrix: Option<Mat4>,
}

impl PerspectiveCamera {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> PerspectiveCameraBuilder {
        PerspectiveCameraBuilder::default()
    }
}

impl CameraExt for PerspectiveCamera {
    fn get_view_matrix(&self) -> Mat4 {
        todo!()
    }

    fn get_projection_matrix(&mut self, aspect_ratio: f32) -> Mat4 {
        if let Some(projection_matrix) = self.projection_matrix {
            projection_matrix
        } else {
            let projection_matrix = Mat4::perspective_rh(
                self.field_of_view_radians,
                aspect_ratio,
                self.z_near_clipping_distance,
                self.z_far_clipping_distance,
            );

            self.projection_matrix = Some(projection_matrix);

            projection_matrix
        }
    }
}

impl ComponentExt for PerspectiveCamera {
    fn id(&self) -> ComponentId {
        self.id
    }
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        PerspectiveCamera::builder().build()
    }
}

pub struct PerspectiveCameraBuilder {
    field_of_view_radians: f32,
    z_near_clipping_distance: f32,
    z_far_clipping_distance: f32,
}

impl PerspectiveCameraBuilder {
    pub fn with_field_of_view_radians(mut self, field_of_view_radians: f32) -> Self {
        self.field_of_view_radians = field_of_view_radians;
        self
    }

    pub fn with_field_of_view_degrees(mut self, field_of_view_degrees: f32) -> Self {
        self.field_of_view_radians = field_of_view_degrees.to_radians();
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

    pub fn build(self) -> PerspectiveCamera {
        PerspectiveCamera {
            id: ComponentId::new(),
            field_of_view_radians: self.field_of_view_radians,
            z_near_clipping_distance: self.z_near_clipping_distance,
            z_far_clipping_distance: self.z_far_clipping_distance,
            projection_matrix: None,
        }
    }
}

impl Default for PerspectiveCameraBuilder {
    fn default() -> Self {
        Self {
            field_of_view_radians: 90f32.to_radians(),
            z_near_clipping_distance: 0.1,
            z_far_clipping_distance: 1000.0,
        }
    }
}
