use crate::error::AppError;
use crate::pipeline::services::orchestration::{FrameContext, ProcessingStep};
use async_trait::async_trait;
use indexmap::IndexMap;
use std::time::Instant;
use tracing::{debug, instrument};

/// A phase represents a logical grouping of processing steps
/// Phases can contain steps, which in turn can have substeps
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PipelinePhase {
    Analysis,
    Learning,
    Decision,
    Execution,
    Journaling,
}

impl PipelinePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            PipelinePhase::Analysis => "Analysis",
            PipelinePhase::Learning => "Learning",
            PipelinePhase::Decision => "Decision",
            PipelinePhase::Execution => "Execution",
            PipelinePhase::Journaling => "Journaling",
        }
    }
}

/// A phase handler manages a group of related steps
#[async_trait]
pub trait PhaseHandler: Send + Sync {
    async fn execute(&mut self, context: &mut FrameContext) -> Result<(), AppError>;
    fn phase(&self) -> PipelinePhase;
    fn name(&self) -> &'static str;
}

/// A processing phase that contains multiple steps
pub struct ProcessingPhase {
    phase: PipelinePhase,
    steps: IndexMap<String, Box<dyn ProcessingStep>>,
    phase_name: String,
}

impl ProcessingPhase {
    pub fn new(phase: PipelinePhase, phase_name: impl Into<String>) -> Self {
        Self {
            phase,
            steps: IndexMap::new(),
            phase_name: phase_name.into(),
        }
    }

    pub fn with_step(mut self, name: impl Into<String>, step: Box<dyn ProcessingStep>) -> Self {
        self.steps.insert(name.into(), step);
        self
    }

    pub fn add_step(&mut self, name: impl Into<String>, step: Box<dyn ProcessingStep>) {
        self.steps.insert(name.into(), step);
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

#[async_trait]
impl PhaseHandler for ProcessingPhase {
    #[instrument(skip(self, context), fields(phase = %self.phase.as_str()))]
    async fn execute(&mut self, context: &mut FrameContext) -> Result<(), AppError> {
        let phase_start = Instant::now();
        debug!(
            "Starting phase '{}' ({}) with {} steps",
            self.phase_name,
            self.phase.as_str(),
            self.steps.len()
        );

        // Record phase entry in context
        context.phase_timings.entry_phase(self.phase.clone());

        for (step_name, step) in &mut self.steps {
            debug!("Executing step '{}' in phase '{}'", step_name, self.phase_name);
            let step_start = Instant::now();
            
            context
                .phase_timings
                .entry_step(self.phase.clone(), step_name.clone());
            
            step.process(context).await?;
            
            let step_duration = step_start.elapsed();
            context
                .phase_timings
                .exit_step(self.phase.clone(), step_name.clone(), step_duration);
            
            debug!(
                "Completed step '{}' in {}us",
                step_name,
                step_duration.as_micros()
            );
        }

        let phase_duration = phase_start.elapsed();
        context
            .phase_timings
            .exit_phase(self.phase.clone(), phase_duration);
        
        debug!(
            "Completed phase '{}' in {}us",
            self.phase_name,
            phase_duration.as_micros()
        );

        Ok(())
    }

    fn phase(&self) -> PipelinePhase {
        self.phase.clone()
    }

    fn name(&self) -> &'static str {
        "ProcessingPhase"
    }
}

/// A composite phase that contains multiple sub-phases
pub struct CompositePhase {
    phase: PipelinePhase,
    subphases: IndexMap<String, Box<dyn PhaseHandler>>,
    phase_name: String,
}

impl CompositePhase {
    pub fn new(phase: PipelinePhase, phase_name: impl Into<String>) -> Self {
        Self {
            phase,
            subphases: IndexMap::new(),
            phase_name: phase_name.into(),
        }
    }

    pub fn with_subphase(
        mut self,
        name: impl Into<String>,
        subphase: Box<dyn PhaseHandler>,
    ) -> Self {
        self.subphases.insert(name.into(), subphase);
        self
    }

    pub fn add_subphase(&mut self, name: impl Into<String>, subphase: Box<dyn PhaseHandler>) {
        self.subphases.insert(name.into(), subphase);
    }
}

#[async_trait]
impl PhaseHandler for CompositePhase {
    #[instrument(skip(self, context), fields(phase = %self.phase.as_str()))]
    async fn execute(&mut self, context: &mut FrameContext) -> Result<(), AppError> {
        let phase_start = Instant::now();
        debug!(
            "Starting composite phase '{}' ({}) with {} subphases",
            self.phase_name,
            self.phase.as_str(),
            self.subphases.len()
        );

        context.phase_timings.entry_phase(self.phase.clone());

        for (subphase_name, subphase) in &mut self.subphases {
            debug!("Executing subphase '{}' in phase '{}'", subphase_name, self.phase_name);
            subphase.execute(context).await?;
        }

        let phase_duration = phase_start.elapsed();
        context
            .phase_timings
            .exit_phase(self.phase.clone(), phase_duration);

        debug!(
            "Completed composite phase '{}' in {}us",
            self.phase_name,
            phase_duration.as_micros()
        );

        Ok(())
    }

    fn phase(&self) -> PipelinePhase {
        self.phase.clone()
    }

    fn name(&self) -> &'static str {
        "CompositePhase"
    }
}
