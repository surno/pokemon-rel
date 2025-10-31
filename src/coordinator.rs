use crate::{
    config::Configuration, error::AppError,
    pipeline::orchestration::processing_pipeline::ProcessingPipeline,
};
use tokio_util::sync::CancellationToken;

struct Coordinator {
    configuration: Configuration,
    cancel_token: CancellationToken,
    pipeline: ProcessingPipeline,
}

impl Coordinator {
    pub fn new(configuration: Configuration, pipeline: ProcessingPipeline) -> Self {
        let cancel_token = CancellationToken::new();
        Self {
            configuration,
            cancel_token,
            pipeline,
        }
    }

    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
}

pub struct CoordinatorBuilder {
    configuration: Configuration,
    pipeline: Option<ProcessingPipeline>,
}

impl CoordinatorBuilder {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,
            pipeline: None,
        }
    }

    pub fn rom_path(mut self, rom_path: String) -> Self {
        self.configuration.rom_path = rom_path;
        self
    }

    pub fn frame_buffer_size(mut self, frame_buffer_size: usize) -> Self {
        self.configuration.frame_buffer_size = frame_buffer_size;
        self
    }

    pub fn action_buffer_size(mut self, action_buffer_size: usize) -> Self {
        self.configuration.action_buffer_size = action_buffer_size;
        self
    }

    pub fn enable_metrics(mut self, enable_metrics: bool) -> Self {
        self.configuration.enable_metrics = enable_metrics;
        self
    }

    pub fn pipeline(mut self, pipeline: ProcessingPipeline) -> Self {
        self.pipeline = Some(pipeline);
        self
    }

    pub fn build(self) -> Result<Coordinator, AppError> {
        let pipeline = self
            .pipeline
            .ok_or_else(|| AppError::Config("Pipeline not configured".to_string()))?;
        Ok(Coordinator::new(self.configuration, pipeline))
    }
}
