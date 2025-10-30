use crate::error::AppError;
use crate::pipeline::services::orchestration::{FrameContext, ProcessingStep};
use async_trait::async_trait;
use std::time::Instant;
use tracing::{debug, instrument};

/// A wrapper that automatically instruments a ProcessingStep with timing and error tracking
pub struct InstrumentedStep<S> {
    inner: S,
    step_name: String,
}

impl<S> InstrumentedStep<S> {
    pub fn new(step: S, step_name: impl Into<String>) -> Self {
        Self {
            inner: step,
            step_name: step_name.into(),
        }
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

#[async_trait]
impl<S> ProcessingStep for InstrumentedStep<S>
where
    S: ProcessingStep,
{
    #[instrument(skip(self, context), fields(step = %self.step_name))]
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError> {
        let start = Instant::now();
        debug!("Starting step: {}", self.step_name);

        let result = self.inner.process(context).await;

        let duration = start.elapsed();
        let duration_us = duration.as_micros() as u64;

        match &result {
            Ok(_) => {
                debug!(
                    "Completed step '{}' successfully in {}us",
                    self.step_name, duration_us
                );
            }
            Err(e) => {
                tracing::error!(
                    "Step '{}' failed after {}us: {}",
                    self.step_name,
                    duration_us,
                    e
                );
            }
        }

        // Note: Phase timing is handled by PhaseHandler wrapper
        // This is just for backward compatibility with FrameMetrics
        // In practice, the specific step type should be recorded by the step itself

        result
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }
}

/// Extension trait to easily wrap steps with instrumentation
pub trait StepInstrumentation: Sized {
    fn instrumented(self, name: impl Into<String>) -> InstrumentedStep<Self>;
}

impl<S> StepInstrumentation for S
where
    S: ProcessingStep,
{
    fn instrumented(self, name: impl Into<String>) -> InstrumentedStep<Self> {
        InstrumentedStep::new(self, name)
    }
}
