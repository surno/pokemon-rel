use crate::error::AppError;
use async_trait::async_trait;

/// Chain of Responsibility pattern for processing pipeline
#[async_trait]
pub trait ProcessingStep<In, Out>: Send + Sync {
    async fn process(&mut self, context: &mut In) -> Result<Out, AppError>;
    fn name(&self) -> &'static str;
}
