use crate::error::AppError;
use crate::pipeline::services::{
    image::analysis::SceneAnalysisOrchestrator,
    learning::smart_action_service::SmartActionService,
    orchestration::{
        ProcessingStep,
        frame_context::{FrameContext, ProcessingStepType},
    },
};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tower::Service;

/// Processing step that handles scene annotation and situation analysis
pub struct SceneAnalysisStep {
    scene_analysis_orchestrator: SceneAnalysisOrchestrator,
    smart_action_service: Arc<Mutex<SmartActionService>>,
}

impl SceneAnalysisStep {
    pub fn new(
        scene_analysis_orchestrator: SceneAnalysisOrchestrator,
        smart_action_service: Arc<Mutex<SmartActionService>>,
    ) -> Self {
        Self {
            scene_analysis_orchestrator,
            smart_action_service,
        }
    }
}

#[async_trait]
impl ProcessingStep for SceneAnalysisStep {
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError> {
        let step_start = Instant::now();

        // First, annotate the frame with scene detection using new orchestrator
        let annotated_frame = self
            .scene_analysis_orchestrator
            .call(context.frame.clone())
            .await?;
        context.frame = annotated_frame;

        // Then, analyze the situation using the smart action service
        let situation = {
            let smart_service = self.smart_action_service.lock().unwrap();
            smart_service.analyze_situation(&context.frame)
        };

        // Make a decision using the smart action service
        let smart_decision = {
            let mut smart_service = self.smart_action_service.lock().unwrap();
            smart_service.make_decision(&situation)
        };

        // Update context with results
        context.situation = Some(situation);
        context.smart_decision = Some(smart_decision);

        // Record timing
        let duration = step_start.elapsed().as_micros() as u64;
        context
            .metrics
            .record_duration(ProcessingStepType::SceneAnalysis, duration);

        Ok(())
    }

    fn name(&self) -> &'static str {
        "SceneAnalysisStep"
    }
}
