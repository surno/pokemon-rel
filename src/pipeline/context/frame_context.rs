use crate::common::frame::Frame;
use crate::pipeline::context::metrics::FrameMetrics;
use crate::pipeline::context::state::AnalyzedState;
use crate::pipeline::context::state::IngestedState;
use crate::pipeline::domain::scene_analysis::SceneAnalysis;
use std::sync::Arc;
use std::time::{Duration, Instant};

// FrameContext with compile-time state tracking via the phantom data
pub struct FrameContext<S> {
    frame: Arc<Frame>,
    metrics: FrameMetrics,
    processing_start: Instant,
    state: S,
}

impl<S> FrameContext<S> {
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn metrics(&self) -> &FrameMetrics {
        &self.metrics
    }

    pub fn elapsed(&self) -> Duration {
        self.processing_start.elapsed()
    }
}

impl FrameContext<IngestedState> {
    pub fn new(frame: Frame) -> Self {
        Self {
            frame: Arc::new(frame),
            metrics: FrameMetrics::new(),
            processing_start: Instant::now(),
            state: IngestedState,
        }
    }

    pub fn into_analyzed(mut self, analysis: SceneAnalysis) -> FrameContext<AnalyzedState> {
        self.metrics.record_analysis_duration(self.elapsed());
        FrameContext::<AnalyzedState> {
            frame: self.frame,
            metrics: self.metrics,
            processing_start: self.processing_start,
            state: AnalyzedState { analysis },
        }
    }
}

impl FrameContext<AnalyzedState> {
    pub fn analysis(&self) -> &SceneAnalysis {
        &self.state.analysis
    }
}
