use std::io;

#[derive(Debug)]
pub enum MeshLoaderError {
    IOError(io::Error),
    WavefrontError(obj::ObjError),
}

impl From<io::Error> for MeshLoaderError {
    fn from(value: io::Error) -> Self {
        Self::IOError(value)
    }
}

impl From<obj::ObjError> for MeshLoaderError {
    fn from(value: obj::ObjError) -> Self {
        Self::WavefrontError(value)
    }
}
