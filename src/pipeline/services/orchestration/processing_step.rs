use super::frame_context::FrameContext;
use crate::error::AppError;
use async_trait::async_trait;

/// Chain of Responsibility pattern for processing pipeline
#[async_trait]
pub trait ProcessingStep: Send + Sync {
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError>;
    fn name(&self) -> &'static str;
}

/// A pipeline that processes frames through a chain of steps
pub struct ProcessingPipeline {
    steps: Vec<Box<dyn ProcessingStep>>,
}

impl ProcessingPipeline {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn add_step(mut self, step: Box<dyn ProcessingStep>) -> Self {
        self.steps.push(step);
        self
    }

    pub async fn process(&mut self, mut context: FrameContext) -> Result<FrameContext, AppError> {
        for step in &mut self.steps {
            tracing::debug!("Processing step: {}", step.name());
            step.process(&mut context).await?;
        }
        Ok(context)
    }
}
