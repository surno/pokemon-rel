use crate::error::AppError;
use crate::pipeline::orchestration::frame_context::FrameContext;
use crate::pipeline::orchestration::processing_step::ProcessingStep;

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
