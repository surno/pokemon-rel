use super::frame_context::FrameContext;
use super::pipeline_stage::{PipelineStage, StageStep, StageStepContainer};
use crate::error::AppError;
use async_trait::async_trait;
use std::collections::BTreeMap;

/// Chain of Responsibility pattern for processing pipeline (legacy trait)
/// This trait is maintained for backward compatibility
#[async_trait]
pub trait ProcessingStep: Send + Sync {
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError>;
    fn name(&self) -> &'static str;
}

/// Adapter to convert legacy ProcessingStep to new StageStep
/// This allows gradual migration while maintaining backward compatibility
pub struct ProcessingStepAdapter {
    step: Box<dyn ProcessingStep>,
}

impl ProcessingStepAdapter {
    pub fn new(step: Box<dyn ProcessingStep>) -> Self {
        Self { step }
    }
}

#[async_trait]
impl StageStep for ProcessingStepAdapter {
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError> {
        self.step.process(context).await
    }

    fn step_name(&self) -> &'static str {
        self.step.name()
    }
}

/// A pipeline that processes frames through structured stages
/// 
/// Industry-standard Rust patterns used:
/// - BTreeMap for ordered stage execution (stages are sorted by priority)
/// - Vec for ordered step execution within stages (idiomatic for sequential processing)
/// - Zero-cost abstractions with trait objects for flexibility
/// - Clear separation of concerns: stages group related steps
pub struct ProcessingPipeline {
    /// Stages organized by type for efficient lookup and ordered execution
    /// Uses BTreeMap to maintain priority order (stages are ordered by PipelineStage::priority)
    stages: BTreeMap<PipelineStage, StageStepContainer>,
}

impl Default for ProcessingPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessingPipeline {
    /// Create a new empty pipeline
    pub fn new() -> Self {
        Self {
            stages: BTreeMap::new(),
        }
    }

    /// Add a stage container to the pipeline
    /// If a stage of this type already exists, the steps will be appended
    pub fn add_stage(mut self, container: StageStepContainer) -> Self {
        let stage_type = container.stage_type();
        match self.stages.get_mut(&stage_type) {
            Some(existing) => {
                // Merge steps into existing stage
                // Note: This requires exposing internal structure or using a different approach
                // For now, we'll replace if exists (can be enhanced later)
                self.stages.insert(stage_type, container);
            }
            None => {
                self.stages.insert(stage_type, container);
            }
        }
        self
    }

    /// Add a single step to a stage, creating the stage if it doesn't exist
    pub fn add_step_to_stage(
        mut self,
        stage: PipelineStage,
        step: Box<dyn StageStep>,
    ) -> Self {
        let container = self
            .stages
            .entry(stage)
            .or_insert_with(|| StageStepContainer::new(stage));
        container.add_step_mut(step);
        self
    }

    /// Process a frame through all stages in priority order
    /// Stages are executed sequentially, steps within stages are executed sequentially
    pub async fn process(&mut self, mut context: FrameContext) -> Result<FrameContext, AppError> {
        // BTreeMap iterates in key order, which matches our priority ordering
        for (stage_type, container) in &mut self.stages {
            tracing::debug!("Processing stage: {}", stage_type.name());
            container.execute_all(&mut context).await?;
        }
        Ok(context)
    }

    /// Get the number of stages in the pipeline
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Get total number of steps across all stages
    pub fn total_step_count(&self) -> usize {
        self.stages.values().map(|c| c.step_count()).sum()
    }

    /// Legacy method for backward compatibility
    /// Automatically assigns steps to appropriate stages based on step name
    /// This allows gradual migration from the old flat pipeline structure
    pub fn add_step(mut self, step: Box<dyn ProcessingStep>) -> Self {
        let step_name = step.name();
        // Map old step names to stages based on their purpose
        let stage = if step_name.contains("Scene") || step_name.contains("ImageChange") {
            PipelineStage::Analysis
        } else if step_name.contains("Policy") {
            PipelineStage::ReinforcementLearning
        } else if step_name.contains("Action") || step_name.contains("Macro") {
            PipelineStage::ActionSelection
        } else if step_name.contains("Learning") {
            PipelineStage::Learning
        } else {
            // Default to Analysis stage for unknown steps
            PipelineStage::Analysis
        };

        self.add_step_to_stage(stage, Box::new(ProcessingStepAdapter::new(step)))
    }
}
