use crate::error::AppError;
use crate::pipeline::services::{
    RLService,
    orchestration::{
        ProcessingStep,
        frame_context::{FrameContext, ProcessingStepType},
    },
};
use async_trait::async_trait;
use std::time::Instant;
use tower::Service;

/// Processing step that handles policy inference using the RL service
pub struct PolicyInferenceStep {
    rl_service: RLService,
}

impl PolicyInferenceStep {
    pub fn new(rl_service: RLService) -> Self {
        Self { rl_service }
    }
}

#[async_trait]
impl ProcessingStep for PolicyInferenceStep {
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError> {
        let step_start = Instant::now();

        // Get policy prediction for the current frame
        let prediction = self.rl_service.call(context.frame.clone()).await?;

        // Update context with prediction
        context.policy_prediction = Some(prediction);

        // Record timing
        let duration = step_start.elapsed().as_micros() as u64;
        context
            .metrics
            .record_duration(ProcessingStepType::PolicyInference, duration);

        Ok(())
    }

    fn name(&self) -> &'static str {
        "PolicyInferenceStep"
    }
}
