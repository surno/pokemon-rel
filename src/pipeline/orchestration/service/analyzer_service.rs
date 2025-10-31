use std::pin::Pin;
use std::sync::Arc;

use crate::pipeline::context::frame_context::FrameContext;
use crate::pipeline::context::state::AnalyzedState;
use crate::pipeline::context::state::IngestedState;
use crate::pipeline::orchestration::processing_pipeline::AnalyzerStep;
use futures::Future;
use futures::task::Context;
use futures::task::Poll;
use tower::Service;

#[derive(Clone)]
pub struct AnalyzerService {
    inner: Arc<dyn AnalyzerStep>,
}

impl AnalyzerService {
    pub fn new(inner: Box<dyn AnalyzerStep>) -> Self {
        Self {
            inner: Arc::from(inner),
        }
    }
}

impl Service<FrameContext<IngestedState>> for AnalyzerService {
    type Response = FrameContext<AnalyzedState>;
    type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: FrameContext<IngestedState>) -> Self::Future {
        let inner = self.inner.clone();

        let future = Box::pin(async move {
            let analysis = inner.analyze(&req).await?;
            Ok(req.into_analyzed(analysis))
        });

        future
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use image::{DynamicImage, ImageBuffer, Rgb};
    use uuid::Uuid;

    use crate::{
        common::Frame,
        pipeline::{
            domain::scene_analysis::SceneType, orchestration::step::scene_analyzer::SceneAnalyzer,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_analyzer_service() {
        let mut analyzer_service = AnalyzerService::new(Box::new(SceneAnalyzer::new()));
        let frame_context = FrameContext::new(Frame::new(
            Uuid::new_v4(),
            DynamicImage::ImageRgb8(ImageBuffer::<Rgb<u8>, Vec<u8>>::from_pixel(
                100,
                100,
                Rgb([255, 255, 255]),
            )),
            Utc::now(),
            Uuid::new_v4(),
        ));
        let response = analyzer_service.call(frame_context).await.unwrap();
        assert!(response.analysis().scene_type() == SceneType::Unknown);
    }
}
