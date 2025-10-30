use crate::pipeline::services::learning::smart_action_service::{ActionDecision, GameSituation};
use crate::pipeline::{EnrichedFrame, GameAction, RLPrediction};
use crate::pipeline::services::orchestration::phase_timings::PhaseTimings;
use std::time::Instant;
use uuid::Uuid;

/// Context object that flows through the processing pipeline
/// Contains all the state needed for processing a single frame
#[derive(Clone)]
pub struct FrameContext {
    pub frame: EnrichedFrame,
    pub client_id: Uuid,
    pub situation: Option<GameSituation>,
    pub smart_decision: Option<ActionDecision>,
    pub policy_prediction: Option<RLPrediction>,
    pub selected_action: Option<GameAction>,
    pub macro_action: Option<crate::pipeline::MacroAction>,
    pub image_changed: bool,
    pub metrics: FrameMetrics,
    pub phase_timings: PhaseTimings,
    pub processing_start: Instant,
}

impl FrameContext {
    pub fn new(frame: EnrichedFrame) -> Self {
        let client_id = frame.client;
        Self {
            frame,
            client_id,
            situation: None,
            smart_decision: None,
            policy_prediction: None,
            selected_action: None,
            macro_action: None,
            image_changed: false,
            metrics: FrameMetrics::new(),
            phase_timings: PhaseTimings::new(),
            processing_start: Instant::now(),
        }
    }
}

/// Metrics collected during frame processing
#[derive(Debug, Clone, Default)]
pub struct FrameMetrics {
    pub scene_analysis_duration_us: u64,
    pub policy_inference_duration_us: u64,
    pub action_selection_duration_us: u64,
    pub macro_execution_duration_us: u64,
    pub reward_processing_duration_us: u64,
    pub experience_collection_duration_us: u64,
    pub image_change_detection_us: u64,
    pub total_processing_duration_us: u64,
}

impl FrameMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_duration(&mut self, step: ProcessingStepType, duration_us: u64) {
        match step {
            ProcessingStepType::SceneAnalysis => self.scene_analysis_duration_us = duration_us,
            ProcessingStepType::PolicyInference => self.policy_inference_duration_us = duration_us,
            ProcessingStepType::ActionSelection => self.action_selection_duration_us = duration_us,
            ProcessingStepType::MacroExecution => self.macro_execution_duration_us = duration_us,
            ProcessingStepType::RewardProcessing => {
                self.reward_processing_duration_us = duration_us
            }
            ProcessingStepType::ExperienceCollection => {
                self.experience_collection_duration_us = duration_us
            }
            ProcessingStepType::ImageChangeDetection => {
                self.image_change_detection_us = duration_us
            }
            ProcessingStepType::ActionSending => {}
        }
    }

    pub fn finalize(&mut self, start_time: Instant) {
        self.total_processing_duration_us = start_time.elapsed().as_micros() as u64;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessingStepType {
    SceneAnalysis,
    PolicyInference,
    ActionSelection,
    MacroExecution,
    RewardProcessing,
    ExperienceCollection,
    ImageChangeDetection,
    ActionSending,
}
