use std::pin::Pin;

use crate::common::Frame;
use crate::error::AppError;
use crate::pipeline::context::frame_context::FrameContext;
use crate::pipeline::context::state::{AnalyzedState, IngestedState};
use crate::pipeline::domain::scene_analysis::SceneAnalysis;
use crate::pipeline::orchestration::step::scene_analyzer::AnalyzerBuilder;
use async_trait::async_trait;
use time::Duration;
use tower::Service;

pub struct ProcessingPipeline {
    pub enable_metrics: bool,
    pub analyzer_step: Box<
        dyn Service<
                FrameContext<IngestedState>,
                Response = FrameContext<AnalyzedState>,
                Error = Box<dyn std::error::Error + Send + Sync + 'static>,
                Future = Pin<
                    Box<
                        dyn Future<
                                Output = Result<
                                    FrameContext<AnalyzedState>,
                                    Box<dyn std::error::Error + Send + Sync + 'static>,
                                >,
                            > + Send
                            + 'static,
                    >,
                >,
            >,
    >,
}

impl ProcessingPipeline {
    pub fn builder() -> ProcessingPipelineBuilder {
        ProcessingPipelineBuilder::new()
    }

    pub async fn process(&mut self, frame: Frame) -> Result<FrameContext<AnalyzedState>, AppError> {
        let frame_context = FrameContext::new(frame);
        let response = self.analyzer_step.call(frame_context).await?;
        Ok(response)
    }
}

pub struct ProcessingPipelineBuilder {
    pub timeout: Option<Duration>,
    pub rate_limit: Option<(u64, Duration)>,
    pub enable_metrics: bool,
}

impl ProcessingPipelineBuilder {
    pub fn new() -> Self {
        Self {
            timeout: None,
            rate_limit: None,
            enable_metrics: false,
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn rate_limit(mut self, rate_limit: (u64, Duration)) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }

    pub fn enable_metrics(mut self, enable_metrics: bool) -> Self {
        self.enable_metrics = enable_metrics;
        self
    }

    pub fn add_analyzer(self, analyzer: Box<dyn AnalyzerStep>) -> AnalyzerBuilder {
        AnalyzerBuilder {
            config: self,
            analyzer_timeout: None,
            analyzer,
        }
    }
}

impl Default for ProcessingPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Trait for analyzer implementations
#[async_trait]
pub trait AnalyzerStep: Send + Sync + 'static {
    async fn analyze(&self, ctx: &FrameContext<IngestedState>) -> Result<SceneAnalysis, AppError>;
}
