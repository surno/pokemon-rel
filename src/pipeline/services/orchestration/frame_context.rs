use crate::error::AppError;
use crate::pipeline::services::learning::smart_action_service::{ActionDecision, GameSituation};
use crate::pipeline::{EnrichedFrame, GameAction, RLPrediction};
use std::collections::BTreeMap;
use std::time::Instant;
use uuid::Uuid;

/// Context object that flows through the processing pipeline
/// Contains all the state needed for processing a single frame
/// Uses BTreeMap for ordered, extensible metadata storage
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
    pub processing_start: Instant,
    /// Extensible metadata storage for step-specific data
    /// Uses BTreeMap for ordered iteration and predictable ordering
    pub metadata: BTreeMap<String, String>,
    /// Track step execution status for debugging and observability
    pub step_execution_log: BTreeMap<&'static str, StepExecutionStatus>,
}

/// Execution status for a processing step
/// Uses u64 for timestamps (microseconds since processing start) to enable cloning
#[derive(Debug, Clone)]
pub enum StepExecutionStatus {
    Started { timestamp_us: u64 },
    Completed { duration_us: u64 },
    Error { error: String },
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
            processing_start: Instant::now(),
            metadata: BTreeMap::new(),
            step_execution_log: BTreeMap::new(),
        }
    }
    
    /// Mark a step as started
    pub fn mark_step_start(&mut self, step_name: &'static str) {
        let timestamp_us = self.processing_start.elapsed().as_micros() as u64;
        self.step_execution_log.insert(
            step_name,
            StepExecutionStatus::Started { timestamp_us },
        );
    }
    
    /// Mark a step as completed
    pub fn mark_step_complete(&mut self, step_name: &'static str) {
        if let Some(StepExecutionStatus::Started { timestamp_us: start_us }) = self.step_execution_log.get(step_name) {
            let current_us = self.processing_start.elapsed().as_micros() as u64;
            let duration_us = current_us.saturating_sub(*start_us);
            self.step_execution_log.insert(
                step_name,
                StepExecutionStatus::Completed { duration_us },
            );
        }
    }
    
    /// Mark a step as errored
    pub fn mark_step_error(&mut self, step_name: &'static str, error: &AppError) {
        self.step_execution_log.insert(
            step_name,
            StepExecutionStatus::Error {
                error: error.to_string(),
            },
        );
    }
    
    /// Store arbitrary metadata (useful for extensibility)
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
    
    /// Retrieve metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
    
    /// Check if a step has completed successfully
    pub fn step_completed(&self, step_name: &str) -> bool {
        matches!(
            self.step_execution_log.get(step_name),
            Some(StepExecutionStatus::Completed { .. })
        )
    }
}

/// Metrics collected during frame processing
/// Uses HashMap for flexible, extensible step timing storage
#[derive(Debug, Clone)]
pub struct FrameMetrics {
    /// Duration per step type using HashMap for O(1) lookups
    /// More flexible than individual fields and easier to extend
    step_durations: std::collections::HashMap<ProcessingStepType, u64>,
    /// Total processing duration
    pub total_processing_duration_us: u64,
    /// Legacy fields for backward compatibility (computed from step_durations)
    pub scene_analysis_duration_us: u64,
    pub policy_inference_duration_us: u64,
    pub action_selection_duration_us: u64,
    pub macro_execution_duration_us: u64,
    pub reward_processing_duration_us: u64,
    pub experience_collection_duration_us: u64,
    pub image_change_detection_us: u64,
}

impl Default for FrameMetrics {
    fn default() -> Self {
        Self {
            step_durations: std::collections::HashMap::new(),
            total_processing_duration_us: 0,
            scene_analysis_duration_us: 0,
            policy_inference_duration_us: 0,
            action_selection_duration_us: 0,
            macro_execution_duration_us: 0,
            reward_processing_duration_us: 0,
            experience_collection_duration_us: 0,
            image_change_detection_us: 0,
        }
    }
}

impl FrameMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record duration for a processing step
    /// Uses HashMap for efficient storage and lookup
    pub fn record_duration(&mut self, step: ProcessingStepType, duration_us: u64) {
        // Store in HashMap
        self.step_durations.insert(step, duration_us);
        
        // Update legacy fields for backward compatibility
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
    
    /// Get duration for a specific step (O(1) lookup)
    pub fn get_step_duration(&self, step: ProcessingStepType) -> Option<u64> {
        self.step_durations.get(&step).copied()
    }
    
    /// Get all step durations (useful for metrics aggregation)
    pub fn all_step_durations(&self) -> &std::collections::HashMap<ProcessingStepType, u64> {
        &self.step_durations
    }
    
    /// Calculate total duration across all recorded steps
    pub fn total_step_duration(&self) -> u64 {
        self.step_durations.values().sum()
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
