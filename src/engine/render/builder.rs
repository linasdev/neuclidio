use crate::engine::render::RenderEngine;
use crate::error::NeuclidioResult;
use vulkanalia::vk;
use vulkanalia::vk::HasBuilder;

pub struct RenderEngineBuilder {
    pub application_name: String,
    pub application_version_major: u32,
    pub application_version_minor: u32,
    pub application_version_patch: u32,
}

impl Default for RenderEngineBuilder {
    fn default() -> Self {
        Self {
            application_name: "Neuclidio Example".to_string(),
            application_version_major: 1,
            application_version_minor: 0,
            application_version_patch: 0,
        }
    }
}

impl RenderEngineBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_application_name(mut self, name: impl Into<String>) -> Self {
        self.application_name = name.into();
        self
    }

    pub fn with_application_version(
        mut self,
        version_major: u32,
        version_minor: u32,
        version_patch: u32,
    ) -> Self {
        self.application_version_major = version_major.into();
        self.application_version_minor = version_minor.into();
        self.application_version_patch = version_patch.into();
        self
    }

    pub fn build(self) -> NeuclidioResult<RenderEngine> {
        let application_info = vk::ApplicationInfo::builder()
            .application_name((self.application_name + "\0").as_bytes())
            .application_version(vk::make_version(
                self.application_version_major,
                self.application_version_minor,
                self.application_version_patch,
            ))
            .engine_name(b"Neuclidio\0")
            .engine_version(vk::make_version(
                env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
                env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
                env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
            ))
            .api_version(vk::make_version(1, 3, 0))
            .build();

        Ok(RenderEngine::new(application_info))
    }
}
