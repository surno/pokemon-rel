pub mod action_selector;
pub mod frame_context;
pub mod metrics;
pub mod pipeline_orchestrator;
pub mod pipeline_v2;
pub mod pipeline_v2_factory;
pub mod processing_step;
pub mod step_adapter;
pub mod ui_adapter;

pub use action_selector::{ActionSelection, ActionSelector};
pub use frame_context::{FrameContext, FrameMetrics};
pub use metrics::{MetricsCollector, MetricsObserver};
pub use pipeline_orchestrator::AIPipelineOrchestrator;
pub use pipeline_v2::{
    CompositeStep, FrameMetricsV2, PipelineStage, ProcessingPhase, ProcessingStepV2,
    StepAccumulator, StepContext, StepResult, StagedProcessingPipeline,
};
pub use processing_step::{ProcessingPipeline, ProcessingStep};
pub use step_adapter::StepAdapter;
pub use ui_adapter::UIPipelineAdapter;
