use crate::component::mesh::Mesh;
use crate::component::mesh::loader::error::MeshLoaderError;
use crate::engine::render::pipeline::common::vertex::Vertex;
use crate::error::NeuclidioResult;
use crate::id_generator::IdGenerator;
use glam::{Vec2, Vec3};
use obj::{Obj, load_obj};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

pub mod error;

pub struct MeshLoader {}

impl MeshLoader {
    pub fn load_mesh_from_file_wavefront(mesh_path: impl AsRef<Path>) -> NeuclidioResult<Mesh> {
        let mesh_file = File::open(mesh_path).map_err(MeshLoaderError::from)?;

        let input = BufReader::new(mesh_file);
        let wavefront_object: Obj = load_obj(input).map_err(MeshLoaderError::from)?;

        let vertices = wavefront_object
            .vertices
            .into_iter()
            .map(|vertex| vertex.into())
            .collect();

        let indices = wavefront_object
            .indices
            .into_iter()
            .map(|index| index as u32)
            .collect();

        let mesh = Mesh {
            id: IdGenerator::generate_mesh_id(),
            vertices: Arc::new(vertices),
            indices: Arc::new(indices),
        };

        Ok(mesh)
    }
}

impl From<obj::Vertex> for Vertex {
    fn from(value: obj::Vertex) -> Self {
        let position = Vec3::new(value.position[0], value.position[1], value.position[2]);
        let normal = Vec3::new(value.normal[0], value.normal[1], value.normal[2]);
        Self::new(position, normal, Vec2::ZERO)
    }
}
