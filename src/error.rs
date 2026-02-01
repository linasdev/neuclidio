use crate::windowing::NeuclidioWindowingError;

#[derive(Debug)]
pub enum NeuclidioError {
    WindowingError(NeuclidioWindowingError),
}
