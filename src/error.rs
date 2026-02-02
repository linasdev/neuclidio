use crate::windowing::error::NeuclidioWindowingError;

#[derive(Debug)]
pub enum NeuclidioError {
    EventBusClosed,
    WindowingError(NeuclidioWindowingError),
}

impl From<NeuclidioWindowingError> for NeuclidioError {
    fn from(value: NeuclidioWindowingError) -> Self {
        Self::WindowingError(value)
    }
}
