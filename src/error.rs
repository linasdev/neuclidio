use crate::component::error::ComponentError;
use crate::engine::proxy::error::EngineProxyError;
use crate::engine::render::error::RenderError;

pub type NeuclidioResult<T> = Result<T, NeuclidioError>;

#[derive(Debug)]
pub enum NeuclidioError {
    EngineAlreadyExists,
    EngineProxyError(EngineProxyError),
    RenderError(RenderError),
    ComponentError(ComponentError),
}

impl From<EngineProxyError> for NeuclidioError {
    fn from(value: EngineProxyError) -> Self {
        Self::EngineProxyError(value)
    }
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
