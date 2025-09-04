use crate::error::AppError;
use crate::pipeline::services::{
    managers::{ClientStateManager, ImageChangeDetector},
    orchestration::{
        ProcessingStep,
        frame_context::{FrameContext, ProcessingStepType},
    },
};
use async_trait::async_trait;
use std::time::Instant;

/// Processing step that handles image change detection and client state management
pub struct ImageChangeDetectionStep {
    image_change_detector: ImageChangeDetector,
    client_state_manager: ClientStateManager,
}

impl ImageChangeDetectionStep {
    pub fn new(
        image_change_detector: ImageChangeDetector,
        client_state_manager: ClientStateManager,
    ) -> Self {
        Self {
            image_change_detector,
            client_state_manager,
        }
    }
}

#[async_trait]
impl ProcessingStep for ImageChangeDetectionStep {
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError> {
        let step_start = Instant::now();

        // Detect image changes
        let image_changed = self
            .image_change_detector
            .detect_change(context.client_id, &context.frame.image);
        context.image_changed = image_changed;

        // Update client state if we have the necessary information
        if let (Some(situation), Some(selected_action)) =
            (context.situation.as_ref(), context.selected_action.as_ref())
        {
            // Create small image for caching
            let small_image =
                context
                    .frame
                    .image
                    .resize(64, 64, image::imageops::FilterType::Nearest);

            // Update client state
            self.client_state_manager.update_client_state(
                context.client_id,
                *selected_action,
                situation.clone(),
                small_image,
            );

            // Update intro scene tracking
            self.client_state_manager
                .update_intro_tracking(context.client_id, situation.scene);

            // Add decision to history if available
            if let Some(smart_decision) = context.smart_decision.as_ref() {
                self.client_state_manager
                    .add_decision_to_history(context.client_id, smart_decision.clone());
            }
        }

        // Record timing
        let duration = step_start.elapsed().as_micros() as u64;
        context
            .metrics
            .record_duration(ProcessingStepType::ImageChangeDetection, duration);

        Ok(())
    }

    fn name(&self) -> &'static str {
        "ImageChangeDetectionStep"
    }
}
