use crate::engine::render::pipeline::error::RenderPipelineError;
use crate::engine::render::windowing::error::WindowingError;
use crate::error::NeuclidioError;
use vulkanalia::loader::LoaderError;
use vulkanalia::vk;

#[derive(Debug)]
pub enum RenderError {
    WindowingError(WindowingError),
    RenderPipelineError(RenderPipelineError),
    FailedToLoadLibrary(libloading::Error),
    FailedToCreateVulkanEntry(Box<dyn LoaderError>),
    VulkanErrorCode(vk::ErrorCode),
    NoSuitableDevice,
    MissingRequiredQueueFamilies,
    MissingSwapChainSupport,
    MissingDeviceExtensions,
    MissingSurfaceFormat,
    MissingImageFormat,
    MissingPresentMode,
    MissingValidationLayer,
    OutOfDateSwapChain,
}

impl From<WindowingError> for RenderError {
    fn from(value: WindowingError) -> Self {
        Self::WindowingError(value)
    }
}

impl From<WindowingError> for NeuclidioError {
    fn from(value: WindowingError) -> Self {
        RenderError::from(value).into()
    }
}

impl From<RenderPipelineError> for RenderError {
    fn from(value: RenderPipelineError) -> Self {
        Self::RenderPipelineError(value)
    }
}

impl From<RenderPipelineError> for NeuclidioError {
    fn from(value: RenderPipelineError) -> Self {
        RenderError::from(value).into()
    }
}

impl From<libloading::Error> for RenderError {
    fn from(value: libloading::Error) -> Self {
        Self::FailedToLoadLibrary(value)
    }
}

impl From<libloading::Error> for NeuclidioError {
    fn from(value: libloading::Error) -> Self {
        RenderError::from(value).into()
    }
}

impl From<Box<dyn LoaderError>> for RenderError {
    fn from(value: Box<dyn LoaderError>) -> Self {
        Self::FailedToCreateVulkanEntry(value)
    }
}

impl From<Box<dyn LoaderError>> for NeuclidioError {
    fn from(value: Box<dyn LoaderError>) -> Self {
        RenderError::from(value).into()
    }
}

impl From<vk::ErrorCode> for RenderError {
    fn from(value: vk::ErrorCode) -> Self {
        Self::VulkanErrorCode(value)
    }
}

impl From<vk::ErrorCode> for NeuclidioError {
    fn from(value: vk::ErrorCode) -> Self {
        RenderError::from(value).into()
    }
}
