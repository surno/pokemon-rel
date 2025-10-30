use super::frame_context::FrameContext;
use super::pipeline_phase::{PhaseHandler, PipelinePhase};
use crate::error::AppError;
use async_trait::async_trait;
use indexmap::IndexMap;

/// Chain of Responsibility pattern for processing pipeline
#[async_trait]
pub trait ProcessingStep: Send + Sync {
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError>;
    fn name(&self) -> &'static str;
}

/// A pipeline that processes frames through a chain of steps
/// This maintains backward compatibility with the existing step-based approach
pub struct ProcessingPipeline {
    steps: Vec<Box<dyn ProcessingStep>>,
}

impl ProcessingPipeline {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn add_step(mut self, step: Box<dyn ProcessingStep>) -> Self {
        self.steps.push(step);
        self
    }

    pub async fn process(&mut self, mut context: FrameContext) -> Result<FrameContext, AppError> {
        for step in &mut self.steps {
            tracing::debug!("Processing step: {}", step.name());
            step.process(&mut context).await?;
        }
        Ok(context)
    }
}

/// A hierarchical pipeline that processes frames through phases
/// This is the new recommended approach using industry-standard Rust patterns
pub struct HierarchicalPipeline {
    phases: IndexMap<PipelinePhase, Box<dyn PhaseHandler>>,
}

impl HierarchicalPipeline {
    pub fn new() -> Self {
        Self {
            phases: IndexMap::new(),
        }
    }

    pub fn with_phase(mut self, phase: PipelinePhase, handler: Box<dyn PhaseHandler>) -> Self {
        self.phases.insert(phase, handler);
        self
    }

    pub fn add_phase(&mut self, phase: PipelinePhase, handler: Box<dyn PhaseHandler>) {
        self.phases.insert(phase, handler);
    }

    pub async fn process(&mut self, mut context: FrameContext) -> Result<FrameContext, AppError> {
        // Process phases in insertion order (IndexMap preserves order)
        for (phase, handler) in &mut self.phases {
            tracing::debug!("Processing phase: {:?}", phase);
            handler.execute(&mut context).await?;
        }
        Ok(context)
    }

    pub fn phase_count(&self) -> usize {
        self.phases.len()
    }
}
