use crate::error::AppError;
use crate::pipeline::context::frame_context::FrameContext;
use crate::pipeline::context::state::IngestedState;
use crate::pipeline::domain::scene_analysis::SceneAnalysis;
use crate::pipeline::domain::scene_analysis::SceneType;
use crate::pipeline::orchestration::processing_pipeline::AnalyzerStep;
use crate::pipeline::orchestration::processing_pipeline::ProcessingPipeline;
use crate::pipeline::orchestration::processing_pipeline::ProcessingPipelineBuilder;
use crate::pipeline::orchestration::service::analyzer_service::AnalyzerService;
use async_trait::async_trait;
use std::time::Duration;
use tower::ServiceBuilder;
use tower::timeout::TimeoutLayer;
use tower::util::BoxService;

pub struct AnalyzerBuilder {
    pub config: ProcessingPipelineBuilder,
    pub analyzer_timeout: Option<Duration>,
    pub analyzer: Box<dyn AnalyzerStep>,
}

impl AnalyzerBuilder {
    pub fn analyzer_timeout(mut self, analyzer_timeout: Duration) -> Self {
        self.analyzer_timeout = Some(analyzer_timeout);
        self
    }

    pub fn build(self) -> ProcessingPipeline {
        let analyzer_builder = ServiceBuilder::new()
            .option_layer(self.analyzer_timeout.map(TimeoutLayer::new))
            .service(AnalyzerService::new(self.analyzer));

        ProcessingPipeline {
            enable_metrics: self.config.enable_metrics,
            analyzer_step: Box::new(BoxService::new(analyzer_builder)),
        }
    }
}

pub struct SceneAnalyzer {
    confidence_threshold: f32,
}

impl SceneAnalyzer {
    pub fn new() -> Self {
        Self {
            confidence_threshold: 0.8,
        }
    }

    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold;
        self
    }
}

#[async_trait]
impl AnalyzerStep for SceneAnalyzer {
    async fn analyze(&self, ctx: &FrameContext<IngestedState>) -> Result<SceneAnalysis, AppError> {
        Ok(SceneAnalysis::new(SceneType::Unknown, 0.0))
    }
}
