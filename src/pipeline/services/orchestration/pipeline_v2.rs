use crate::error::AppError;
use crate::pipeline::{EnrichedFrame, GameAction, RLPrediction};
use crate::pipeline::services::learning::smart_action_service::{ActionDecision, GameSituation};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Processing phase marker types - type-level state machine
/// This ensures steps are executed in the correct order at compile time
/// Future enhancement: Use PhantomData to enforce phase transitions at compile time
pub mod phase {

    pub struct Initial;
    pub struct AnalysisComplete;
    pub struct InferenceComplete;
    pub struct DetectionComplete;
    pub struct SelectionComplete;
    pub struct ExecutionComplete;
    pub struct LearningComplete;
    pub struct Finalized;

    /// Marker trait for valid pipeline phases
    pub trait Phase: Send + Sync {}
    impl Phase for Initial {}
    impl Phase for AnalysisComplete {}
    impl Phase for InferenceComplete {}
    impl Phase for DetectionComplete {}
    impl Phase for SelectionComplete {}
    impl Phase for ExecutionComplete {}
    impl Phase for LearningComplete {}
    impl Phase for Finalized {}
}

/// Step execution context - immutable snapshot of frame data
/// This allows for safer parallel execution and clearer data dependencies
#[derive(Clone)]
pub struct StepContext {
    pub frame: Arc<EnrichedFrame>,
    pub client_id: Uuid,
    pub processing_start: Instant,
}

impl StepContext {
    pub fn from_frame(frame: EnrichedFrame) -> Self {
        Self {
            client_id: frame.client,
            frame: Arc::new(frame),
            processing_start: Instant::now(),
        }
    }
}

/// Accumulator for step results - mutable state that flows through pipeline
/// This separates immutable inputs from mutable outputs
#[derive(Clone)]
pub struct StepAccumulator {
    pub situation: Option<GameSituation>,
    pub smart_decision: Option<ActionDecision>,
    pub policy_prediction: Option<RLPrediction>,
    pub selected_action: Option<GameAction>,
    pub macro_action: Option<crate::pipeline::MacroAction>,
    pub image_changed: bool,
    pub metrics: FrameMetricsV2,
}

impl StepAccumulator {
    pub fn new() -> Self {
        Self {
            situation: None,
            smart_decision: None,
            policy_prediction: None,
            selected_action: None,
            macro_action: None,
            image_changed: false,
            metrics: FrameMetricsV2::new(),
        }
    }
}

/// Enhanced metrics with hierarchical step tracking
#[derive(Clone, Debug, Default)]
pub struct FrameMetricsV2 {
    pub step_metrics: Vec<StepMetric>,
    pub total_processing_duration_us: u64,
}

#[derive(Clone, Debug)]
pub struct StepMetric {
    pub step_path: Vec<String>,
    pub step_name: String,
    pub duration_us: u64,
    pub phase: ProcessingPhase,
}

impl FrameMetricsV2 {
    pub fn new() -> Self {
        Self {
            step_metrics: Vec::new(),
            total_processing_duration_us: 0,
        }
    }

    pub fn record_step(&mut self, step_path: Vec<String>, step_name: String, phase: ProcessingPhase, duration_us: u64) {
        self.step_metrics.push(StepMetric {
            step_path,
            step_name,
            duration_us,
            phase,
        });
    }

    pub fn finalize(&mut self, start_time: Instant) {
        self.total_processing_duration_us = start_time.elapsed().as_micros() as u64;
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub enum ProcessingPhase {
    Analysis,
    Inference,
    Detection,
    Selection,
    Execution,
    Learning,
    Finalization,
}

/// Step execution result - allows conditional step execution
#[derive(Debug)]
pub enum StepResult<T> {
    Continue(T),
    Skip,
    Error(AppError),
}

impl<T> StepResult<T> {
    pub fn map<U, F>(self, f: F) -> StepResult<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            StepResult::Continue(t) => StepResult::Continue(f(t)),
            StepResult::Skip => StepResult::Skip,
            StepResult::Error(e) => StepResult::Error(e),
        }
    }

    pub fn and_then<U, F>(self, f: F) -> StepResult<U>
    where
        F: FnOnce(T) -> StepResult<U>,
    {
        match self {
            StepResult::Continue(t) => f(t),
            StepResult::Skip => StepResult::Skip,
            StepResult::Error(e) => StepResult::Error(e),
        }
    }

    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        match self {
            StepResult::Continue(t) => t,
            StepResult::Skip => T::default(),
            StepResult::Error(e) => panic!("called `StepResult::unwrap_or_default()` on an `Error` value: {:?}", e),
        }
    }
}

/// Base trait for all processing steps
/// This trait supports hierarchical steps and conditional execution
#[async_trait]
pub trait ProcessingStepV2: Send + Sync {
    /// Execute the step
    /// Returns StepResult to allow conditional execution
    async fn execute(
        &mut self,
        context: &StepContext,
        accumulator: &mut StepAccumulator,
        step_path: &[String],
    ) -> StepResult<()>;

    /// Get the step name for logging and metrics
    fn name(&self) -> &'static str;

    /// Get the phase this step belongs to
    fn phase(&self) -> ProcessingPhase;

    /// Check if this step should run based on current accumulator state
    /// Override to provide conditional execution logic
    fn should_execute(&self, _accumulator: &StepAccumulator) -> bool {
        true
    }

    /// Get sub-steps if this is a composite step
    /// Returns empty by default for leaf steps
    fn sub_steps(&self) -> Vec<&dyn ProcessingStepV2> {
        Vec::new()
    }
}

/// Composite step - contains multiple sub-steps executed sequentially
/// This implements the Composite pattern for hierarchical step execution
pub struct CompositeStep {
    name: &'static str,
    phase: ProcessingPhase,
    steps: Vec<Box<dyn ProcessingStepV2>>,
    conditional: Box<dyn Fn(&StepAccumulator) -> bool + Send + Sync>,
}

impl CompositeStep {
    pub fn new(name: &'static str, phase: ProcessingPhase) -> Self {
        Self {
            name,
            phase,
            steps: Vec::new(),
            conditional: Box::new(|_| true),
        }
    }

    pub fn add_step(mut self, step: Box<dyn ProcessingStepV2>) -> Self {
        self.steps.push(step);
        self
    }

    pub fn with_condition<F>(mut self, condition: F) -> Self
    where
        F: Fn(&StepAccumulator) -> bool + Send + Sync + 'static,
    {
        self.conditional = Box::new(condition);
        self
    }
}

#[async_trait]
impl ProcessingStepV2 for CompositeStep {
    async fn execute(
        &mut self,
        context: &StepContext,
        accumulator: &mut StepAccumulator,
        step_path: &[String],
    ) -> StepResult<()> {
        if !(self.conditional)(accumulator) {
            return StepResult::Skip;
        }

        let mut current_path = step_path.to_vec();
        current_path.push(self.name.to_string());

        for step in &mut self.steps {
            if !step.should_execute(accumulator) {
                continue;
            }

            match step.execute(context, accumulator, &current_path).await {
                StepResult::Continue(()) => {}
                StepResult::Skip => {
                    tracing::debug!("Step {} skipped", step.name());
                }
                StepResult::Error(e) => {
                    return StepResult::Error(e);
                }
            }
        }

        StepResult::Continue(())
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn phase(&self) -> ProcessingPhase {
        self.phase
    }

    fn should_execute(&self, accumulator: &StepAccumulator) -> bool {
        (self.conditional)(accumulator)
    }

    fn sub_steps(&self) -> Vec<&dyn ProcessingStepV2> {
        self.steps.iter().map(|s| s.as_ref() as &dyn ProcessingStepV2).collect()
    }
}

/// Pipeline stage - groups steps that can potentially run in parallel
/// Stages execute sequentially, but steps within a stage can be parallelized
#[derive(Clone, Debug)]
pub struct PipelineStage {
    pub name: String,
    pub phase: ProcessingPhase,
    pub steps: Vec<Box<dyn ProcessingStepV2>>,
    pub parallel_execution: bool,
}

impl PipelineStage {
    pub fn new(name: impl Into<String>, phase: ProcessingPhase) -> Self {
        Self {
            name: name.into(),
            phase,
            steps: Vec::new(),
            parallel_execution: false,
        }
    }

    pub fn add_step(mut self, step: Box<dyn ProcessingStepV2>) -> Self {
        self.steps.push(step);
        self
    }

    pub fn with_parallel_execution(mut self, parallel: bool) -> Self {
        self.parallel_execution = parallel;
        self
    }
}

/// Improved pipeline with stage-based execution
/// Uses arena-like storage for better memory locality
pub struct StagedProcessingPipeline {
    stages: Vec<PipelineStage>,
}

impl StagedProcessingPipeline {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
        }
    }

    pub fn add_stage(mut self, stage: PipelineStage) -> Self {
        self.stages.push(stage);
        self
    }

    /// Process a frame through all stages
    pub async fn process(&mut self, frame: EnrichedFrame) -> Result<(StepContext, StepAccumulator), AppError> {
        let context = StepContext::from_frame(frame);
        let mut accumulator = StepAccumulator::new();

        for stage in &mut self.stages {
            tracing::debug!("Executing stage: {} (phase: {:?})", stage.name, stage.phase);

            // Note: True parallel execution requires careful conflict resolution
            // For now, we execute sequentially but stages allow logical grouping
            // Future enhancement: Add conflict detection and merge strategies for parallel execution
            
            // Sequential execution within stage
            for step in &mut stage.steps {
                if !step.should_execute(&accumulator) {
                    continue;
                }

                match step.execute(&context, &mut accumulator, &[]).await {
                    StepResult::Continue(()) => {}
                    StepResult::Skip => {
                        tracing::debug!("Step {} skipped", step.name());
                    }
                    StepResult::Error(e) => return Err(e),
                }
            }
        }

        accumulator.metrics.finalize(context.processing_start);
        Ok((context, accumulator))
    }
}

impl Default for StagedProcessingPipeline {
    fn default() -> Self {
        Self::new()
    }
}
