use crate::error::AppError;
use crate::pipeline::services::orchestration::{
    ActionSelector, ProcessingStep,
    frame_context::{FrameContext, ProcessingStepType},
};
use async_trait::async_trait;
use std::time::Instant;

/// Processing step that handles action selection using the configured strategy
pub struct ActionSelectionStep {
    action_selector: Box<dyn ActionSelector>,
}

impl ActionSelectionStep {
    pub fn new(action_selector: Box<dyn ActionSelector>) -> Self {
        Self { action_selector }
    }
}

#[async_trait]
impl ProcessingStep for ActionSelectionStep {
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError> {
        let step_start = Instant::now();

        // We need both situation and smart decision to proceed
        let situation = context.situation.as_ref().ok_or_else(|| {
            AppError::Client("No situation available for action selection".to_string())
        })?;

        let smart_decision = context.smart_decision.as_ref().ok_or_else(|| {
            AppError::Client("No smart decision available for action selection".to_string())
        })?;

        // Select action using the configured strategy
        let action_selection = self.action_selector.select_action(
            context.client_id,
            situation,
            smart_decision,
            context.policy_prediction.as_ref(),
        );

        // Update context with selected action and macro
        context.selected_action = Some(action_selection.game_action);
        context.macro_action = Some(action_selection.macro_action);

        tracing::info!(
            "Selected action {:?} for client {} using {}: {} (confidence: {:.2})",
            action_selection.game_action,
            context.client_id,
            self.action_selector.name(),
            action_selection.reasoning,
            action_selection.confidence
        );

        // Record timing
        let duration = step_start.elapsed().as_micros() as u64;
        context
            .metrics
            .record_duration(ProcessingStepType::ActionSelection, duration);

        Ok(())
    }

    fn name(&self) -> &'static str {
        "ActionSelectionStep"
    }
}
