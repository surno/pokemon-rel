use super::pipeline_v2::{ProcessingPhase, ProcessingStepV2, StepAccumulator, StepContext, StepResult};
use crate::error::AppError;
use crate::pipeline::services::orchestration::{frame_context::FrameContext, ProcessingStep};
use async_trait::async_trait;
use std::time::Instant;

/// Adapter to bridge existing ProcessingStep trait to new ProcessingStepV2
/// This allows gradual migration to the new pipeline architecture
pub struct StepAdapter {
    step: Box<dyn ProcessingStep>,
    phase: ProcessingPhase,
}

impl StepAdapter {
    pub fn new(step: Box<dyn ProcessingStep>, phase: ProcessingPhase) -> Self {
        Self { step, phase }
    }
}

#[async_trait]
impl ProcessingStepV2 for StepAdapter {
    async fn execute(
        &mut self,
        context: &StepContext,
        accumulator: &mut StepAccumulator,
        step_path: &[String],
    ) -> StepResult<()> {
        let step_start = Instant::now();

        // Convert new context/accumulator to old FrameContext format
        let mut old_context = FrameContext::new((*context.frame).clone());
        
        // Restore state from accumulator
        old_context.situation = accumulator.situation.clone();
        old_context.smart_decision = accumulator.smart_decision.clone();
        old_context.policy_prediction = accumulator.policy_prediction.clone();
        old_context.selected_action = accumulator.selected_action;
        old_context.macro_action = accumulator.macro_action.clone();
        old_context.image_changed = accumulator.image_changed;

        // Execute the old step
        match self.step.process(&mut old_context).await {
            Ok(()) => {
                // Extract results back to accumulator
                accumulator.situation = old_context.situation;
                accumulator.smart_decision = old_context.smart_decision;
                accumulator.policy_prediction = old_context.policy_prediction;
                accumulator.selected_action = old_context.selected_action;
                accumulator.macro_action = old_context.macro_action;
                accumulator.image_changed = old_context.image_changed;

                // Record metrics
                let duration_us = step_start.elapsed().as_micros() as u64;
                let mut step_path_vec = step_path.to_vec();
                step_path_vec.push(self.name().to_string());
                accumulator.metrics.record_step(
                    step_path_vec,
                    self.name().to_string(),
                    self.phase(),
                    duration_us,
                );

                StepResult::Continue(())
            }
            Err(e) => StepResult::Error(e),
        }
    }

    fn name(&self) -> &'static str {
        self.step.name()
    }

    fn phase(&self) -> ProcessingPhase {
        self.phase
    }
}
