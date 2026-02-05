use crate::engine::render::pipeline::error::NeuclidioRenderPipelineError;
use crate::engine::render::windowing::error::NeuclidioWindowingError;
use crate::error::NeuclidioError;
use vulkanalia::loader::LoaderError;
use vulkanalia::vk::ErrorCode;

#[derive(Debug)]
pub enum NeuclidioRenderError {
    WindowingError(NeuclidioWindowingError),
    RenderPipelineError(NeuclidioRenderPipelineError),
    FailedToLoadLibrary(libloading::Error),
    FailedToCreateVulkanEntry(Box<dyn LoaderError>),
    VulkanErrorCode(ErrorCode),
    NoSuitableDevice,
    MissingRequiredQueueFamilies,
    MissingSwapChainSupport,
    MissingDeviceExtensions,
    MissingSurfaceFormat,
    MissingPresentMode,
    MissingValidationLayer,
    OutOfDateSwapChain,
}

impl From<NeuclidioWindowingError> for NeuclidioRenderError {
    fn from(value: NeuclidioWindowingError) -> Self {
        Self::WindowingError(value)
    }
}

impl From<NeuclidioWindowingError> for NeuclidioError {
    fn from(value: NeuclidioWindowingError) -> Self {
        NeuclidioRenderError::from(value).into()
    }
}

impl From<NeuclidioRenderPipelineError> for NeuclidioRenderError {
    fn from(value: NeuclidioRenderPipelineError) -> Self {
        Self::RenderPipelineError(value)
    }
}

impl From<NeuclidioRenderPipelineError> for NeuclidioError {
    fn from(value: NeuclidioRenderPipelineError) -> Self {
        NeuclidioRenderError::from(value).into()
    }
}

impl From<ErrorCode> for NeuclidioRenderError {
    fn from(value: ErrorCode) -> Self {
        Self::VulkanErrorCode(value)
    }
}

impl From<ErrorCode> for NeuclidioError {
    fn from(value: ErrorCode) -> Self {
        NeuclidioRenderError::from(value).into()
    }
}
