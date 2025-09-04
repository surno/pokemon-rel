use crate::error::AppError;
use crate::pipeline::services::{
    managers::MacroManager,
    orchestration::{
        ProcessingStep,
        frame_context::{FrameContext, ProcessingStepType},
    },
};
use async_trait::async_trait;
use std::time::Instant;

/// Processing step that handles macro execution and management
pub struct MacroExecutionStep {
    macro_manager: MacroManager,
}

impl MacroExecutionStep {
    pub fn new(macro_manager: MacroManager) -> Self {
        Self { macro_manager }
    }
}

#[async_trait]
impl ProcessingStep for MacroExecutionStep {
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError> {
        let step_start = Instant::now();

        // We need situation and selected action to proceed
        let situation = context.situation.as_ref().ok_or_else(|| {
            AppError::Client("No situation available for macro execution".to_string())
        })?;

        let suggested_action = context.selected_action.as_ref().ok_or_else(|| {
            AppError::Client("No selected action available for macro execution".to_string())
        })?;

        // Execute macro logic - this may override the selected action
        let final_action = self.macro_manager.execute_macro(
            context.client_id,
            situation,
            suggested_action,
            context.image_changed,
        );

        // Update context with the final action (potentially modified by macro logic)
        context.selected_action = Some(final_action);

        // Record timing
        let duration = step_start.elapsed().as_micros() as u64;
        context
            .metrics
            .record_duration(ProcessingStepType::MacroExecution, duration);

        Ok(())
    }

    fn name(&self) -> &'static str {
        "MacroExecutionStep"
    }
}
