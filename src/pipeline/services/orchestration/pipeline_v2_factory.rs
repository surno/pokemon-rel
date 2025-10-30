use super::{
    step_adapter::StepAdapter, CompositeStep, ProcessingPhase, ProcessingStepV2, PipelineStage,
    StagedProcessingPipeline,
};
use crate::error::AppError;
use crate::pipeline::services::orchestration::ProcessingStep;
use crate::pipeline::services::steps::{
    ActionSelectionStep, ImageChangeDetectionStep, LearningStep, MacroExecutionStep,
    PolicyInferenceStep, SceneAnalysisStep,
};
use std::sync::{Arc, Mutex};

/// Factory for creating improved staged pipelines
/// This demonstrates how to use the new architecture with existing steps
pub struct StagedPipelineFactory;

impl StagedPipelineFactory {
    /// Create a staged pipeline using the new architecture
    /// This shows how to organize steps into logical phases
    pub fn create_staged_pipeline(
        scene_analysis_step: SceneAnalysisStep,
        policy_inference_step: PolicyInferenceStep,
        image_change_step: ImageChangeDetectionStep,
        action_selection_step: ActionSelectionStep,
        macro_execution_step: MacroExecutionStep,
        learning_step: LearningStep,
    ) -> Result<StagedProcessingPipeline, AppError> {
        // Convert existing steps to new architecture using adapters
        let analysis_stage = PipelineStage::new("Analysis", ProcessingPhase::Analysis)
            .add_step(Box::new(StepAdapter::new(
                Box::new(scene_analysis_step),
                ProcessingPhase::Analysis,
            )));

        let inference_stage = PipelineStage::new("Inference", ProcessingPhase::Inference)
            .add_step(Box::new(StepAdapter::new(
                Box::new(policy_inference_step),
                ProcessingPhase::Inference,
            )));

        // Detection stage can be a composite with multiple sub-detections
        let detection_stage = PipelineStage::new("Detection", ProcessingPhase::Detection)
            .add_step(Box::new(StepAdapter::new(
                Box::new(image_change_step),
                ProcessingPhase::Detection,
            )));

        let selection_stage = PipelineStage::new("Selection", ProcessingPhase::Selection)
            .add_step(Box::new(StepAdapter::new(
                Box::new(action_selection_step),
                ProcessingPhase::Selection,
            )));

        let execution_stage = PipelineStage::new("Execution", ProcessingPhase::Execution)
            .add_step(Box::new(StepAdapter::new(
                Box::new(macro_execution_step),
                ProcessingPhase::Execution,
            )));

        // Learning stage as a composite - can have multiple learning components
        let learning_composite = CompositeStep::new("Learning", ProcessingPhase::Learning)
            .add_step(Box::new(StepAdapter::new(
                Box::new(learning_step),
                ProcessingPhase::Learning,
            )));

        let learning_stage = PipelineStage::new("Learning", ProcessingPhase::Learning)
            .add_step(Box::new(learning_composite));

        Ok(StagedProcessingPipeline::new()
            .add_stage(analysis_stage)
            .add_stage(inference_stage)
            .add_stage(detection_stage)
            .add_stage(selection_stage)
            .add_stage(execution_stage)
            .add_stage(learning_stage))
    }

    /// Create a pipeline with conditional learning step
    /// Demonstrates conditional execution pattern
    pub fn create_with_conditional_learning(
        scene_analysis_step: SceneAnalysisStep,
        policy_inference_step: PolicyInferenceStep,
        image_change_step: ImageChangeDetectionStep,
        action_selection_step: ActionSelectionStep,
        macro_execution_step: MacroExecutionStep,
        learning_step: LearningStep,
    ) -> Result<StagedProcessingPipeline, AppError> {
        // Create learning composite that only executes if image changed
        let learning_composite = CompositeStep::new("ConditionalLearning", ProcessingPhase::Learning)
            .with_condition(|acc| acc.image_changed)
            .add_step(Box::new(StepAdapter::new(
                Box::new(learning_step),
                ProcessingPhase::Learning,
            )));

        let learning_stage = PipelineStage::new("Learning", ProcessingPhase::Learning)
            .add_step(Box::new(learning_composite));

        Ok(StagedProcessingPipeline::new()
            .add_stage(PipelineStage::new("Analysis", ProcessingPhase::Analysis).add_step(Box::new(
                StepAdapter::new(Box::new(scene_analysis_step), ProcessingPhase::Analysis),
            )))
            .add_stage(PipelineStage::new("Inference", ProcessingPhase::Inference).add_step(
                Box::new(StepAdapter::new(
                    Box::new(policy_inference_step),
                    ProcessingPhase::Inference,
                )),
            ))
            .add_stage(PipelineStage::new("Detection", ProcessingPhase::Detection).add_step(
                Box::new(StepAdapter::new(
                    Box::new(image_change_step),
                    ProcessingPhase::Detection,
                )),
            ))
            .add_stage(PipelineStage::new("Selection", ProcessingPhase::Selection).add_step(
                Box::new(StepAdapter::new(
                    Box::new(action_selection_step),
                    ProcessingPhase::Selection,
                )),
            ))
            .add_stage(PipelineStage::new("Execution", ProcessingPhase::Execution).add_step(
                Box::new(StepAdapter::new(
                    Box::new(macro_execution_step),
                    ProcessingPhase::Execution,
                )),
            ))
            .add_stage(learning_stage))
    }
}
