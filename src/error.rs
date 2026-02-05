use crate::engine::render::error::NeuclidioRenderError;

pub type NeuclidioResult<T> = Result<T, NeuclidioError>;

#[derive(Debug)]
pub enum NeuclidioError {
    EventBusClosed,
    RenderError(NeuclidioRenderError),
}

impl From<NeuclidioRenderError> for NeuclidioError {
    fn from(value: NeuclidioRenderError) -> Self {
        Self::RenderError(value)
    }
}
