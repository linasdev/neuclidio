use crate::component::error::ComponentError;
use crate::engine::render::error::RenderError;

pub type NeuclidioResult<T> = Result<T, NeuclidioError>;

#[derive(Debug)]
pub enum NeuclidioError {
    EventBusClosed,
    RenderError(RenderError),
    ComponentError(ComponentError),
}

impl From<RenderError> for NeuclidioError {
    fn from(value: RenderError) -> Self {
        Self::RenderError(value)
    }
}

impl From<ComponentError> for NeuclidioError {
    fn from(value: ComponentError) -> Self {
        Self::ComponentError(value)
    }
}
