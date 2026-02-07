use crate::component::mesh::loader::error::MeshLoaderError;
use crate::error::NeuclidioError;

#[derive(Debug)]
pub enum ComponentError {
    MeshLoaderError(MeshLoaderError),
}

impl From<MeshLoaderError> for ComponentError {
    fn from(value: MeshLoaderError) -> Self {
        Self::MeshLoaderError(value)
    }
}

impl From<MeshLoaderError> for NeuclidioError {
    fn from(value: MeshLoaderError) -> Self {
        ComponentError::from(value).into()
    }
}
