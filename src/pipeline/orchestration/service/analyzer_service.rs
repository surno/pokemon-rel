use std::pin::Pin;
use std::sync::Arc;

use crate::error::AppError;
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
