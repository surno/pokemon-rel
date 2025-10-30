pub mod action_selector;
pub mod frame_context;
pub mod metrics;
pub mod pipeline_orchestrator;
pub mod pipeline_stage;
pub mod processing_step;
pub mod ui_adapter;

pub use action_selector::{ActionSelection, ActionSelector};
pub use frame_context::{FrameContext, FrameMetrics, ProcessingStepType};
pub use metrics::{MetricsCollector, MetricsObserver};
pub use pipeline_orchestrator::AIPipelineOrchestrator;
pub use pipeline_stage::{
    PipelineStage, PipelineStageProcessor, StageExecutionMetadata, StageStep,
    StageStepContainer,
};
pub use processing_step::{ProcessingPipeline, ProcessingStep, ProcessingStepAdapter};
pub use ui_adapter::UIPipelineAdapter;
