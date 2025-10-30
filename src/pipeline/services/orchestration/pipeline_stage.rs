use crate::error::AppError;
use crate::pipeline::services::orchestration::frame_context::FrameContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;

/// Represents a high-level stage in the processing pipeline
/// Stages group related steps and provide structured execution
/// 
/// Implements Ord based on priority for use in BTreeMap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    /// Frame analysis stage: scene detection, image processing, feature extraction
    Analysis,
    /// Reinforcement learning stage: policy inference, state evaluation
    ReinforcementLearning,
    /// Action selection stage: decision making, macro management
    ActionSelection,
    /// Learning and adaptation stage: reward processing, experience collection, policy updates
    Learning,
}

impl PipelineStage {
    /// Get display name for the stage
    pub fn name(&self) -> &'static str {
        match self {
            PipelineStage::Analysis => "Analysis",
            PipelineStage::ReinforcementLearning => "ReinforcementLearning",
            PipelineStage::ActionSelection => "ActionSelection",
            PipelineStage::Learning => "Learning",
        }
    }

    /// Get the execution order priority (lower executes first)
    pub fn priority(&self) -> u8 {
        match self {
            PipelineStage::Analysis => 1,
            PipelineStage::ReinforcementLearning => 2,
            PipelineStage::ActionSelection => 3,
            PipelineStage::Learning => 4,
        }
    }
}

impl PartialOrd for PipelineStage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PipelineStage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority().cmp(&other.priority())
    }
}

/// Metadata about stage execution
#[derive(Debug, Clone, Default)]
pub struct StageExecutionMetadata {
    /// When the stage started
    pub started_at: Option<Instant>,
    /// Total duration in microseconds
    pub duration_us: u64,
    /// Number of sub-steps executed
    pub sub_steps_executed: usize,
    /// Whether the stage completed successfully
    pub completed: bool,
    /// Custom metadata from stages
    pub custom_metadata: HashMap<String, String>,
}

impl StageExecutionMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_start(&mut self) {
        self.started_at = Some(Instant::now());
        self.completed = false;
    }

    pub fn record_completion(&mut self) {
        if let Some(start) = self.started_at {
            self.duration_us = start.elapsed().as_micros() as u64;
        }
        self.completed = true;
    }

    pub fn increment_sub_steps(&mut self) {
        self.sub_steps_executed += 1;
    }

    pub fn add_metadata(&mut self, key: String, value: String) {
        self.custom_metadata.insert(key, value);
    }
}

/// A stage in the pipeline that can contain multiple steps
/// This provides hierarchical processing with better organization
#[async_trait]
pub trait PipelineStageProcessor: Send + Sync {
    /// Process a frame through this stage
    async fn process_stage(&mut self, context: &mut FrameContext) -> Result<(), AppError>;

    /// Get the stage type this processor handles
    fn stage_type(&self) -> PipelineStage;

    /// Get a human-readable name for this stage processor
    fn name(&self) -> &'static str;

    /// Check if this stage should be executed for the given context
    /// Default implementation returns true (always execute)
    fn should_execute(&self, context: &FrameContext) -> bool {
        let _ = context;
        true
    }
}

/// A step within a stage - represents individual processing units
/// This is a more granular unit than stages
#[async_trait]
pub trait StageStep: Send + Sync {
    /// Process the step
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError>;

    /// Get step name
    fn step_name(&self) -> &'static str;

    /// Optional: Get sub-steps if this step has nested processing
    /// Default implementation returns empty vector
    fn sub_steps(&self) -> Vec<&'static str> {
        Vec::new()
    }
}

/// Container for steps within a stage
/// Uses Vec for ordered execution, which is idiomatic for sequential processing
pub struct StageStepContainer {
    steps: Vec<Box<dyn StageStep>>,
    stage_type: PipelineStage,
}

impl StageStepContainer {
    pub fn new(stage_type: PipelineStage) -> Self {
        Self {
            steps: Vec::new(),
            stage_type,
        }
    }

    pub fn add_step(mut self, step: Box<dyn StageStep>) -> Self {
        self.steps.push(step);
        self
    }

    pub fn add_step_mut(&mut self, step: Box<dyn StageStep>) {
        self.steps.push(step);
    }

    pub fn add_steps(mut self, steps: Vec<Box<dyn StageStep>>) -> Self {
        self.steps.extend(steps);
        self
    }

    pub fn add_steps_mut(&mut self, steps: Vec<Box<dyn StageStep>>) {
        self.steps.extend(steps);
    }

    pub async fn execute_all(&mut self, context: &mut FrameContext) -> Result<(), AppError> {
        let stage_start = Instant::now();

        // Record stage metadata
        let metadata = context
            .stage_metadata
            .entry(self.stage_type)
            .or_insert_with(StageExecutionMetadata::new);
        metadata.record_start();

        // Execute all steps in order
        for step in &mut self.steps {
            tracing::debug!(
                "Executing step '{}' in stage '{}'",
                step.step_name(),
                self.stage_type.name()
            );
            step.process(context).await?;
            metadata.increment_sub_steps();
        }

        // Record completion
        metadata.record_completion();

        let duration_us = stage_start.elapsed().as_micros() as u64;
        tracing::debug!(
            "Stage '{}' completed in {}μs with {} steps",
            self.stage_type.name(),
            duration_us,
            metadata.sub_steps_executed
        );

        Ok(())
    }

    pub fn stage_type(&self) -> PipelineStage {
        self.stage_type
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}
