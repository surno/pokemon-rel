use crate::error::AppError;
use crate::pipeline::types::{EnrichedFrame, GameStateData, RawFrame};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::RwLock;

use tower::Service;

pub struct FrameTranslationService {
    service: Arc<
        RwLock<
            dyn Service<
                    EnrichedFrame,
                    Response = EnrichedFrame,
                    Error = AppError,
                    Future = Pin<Box<dyn Future<Output = Result<EnrichedFrame, AppError>> + Send>>,
                > + Send
                + Sync,
        >,
    >,
}

impl FrameTranslationService {
    pub fn new(
        service: Box<
            dyn Service<
                    EnrichedFrame,
                    Response = EnrichedFrame,
                    Error = AppError,
                    Future = Pin<Box<dyn Future<Output = Result<EnrichedFrame, AppError>> + Send>>,
                > + Send
                + Sync,
        >,
    ) -> Self {
        Self {
            service: Arc::new(RwLock::new(service)),
        }
    }
}

impl Service<RawFrame> for FrameTranslationService {
    type Response = EnrichedFrame;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: RawFrame) -> Self::Future {
        let service = self.service.clone();
        Box::pin(async move {
            let mut service = service.write().await;
            let enriched_frame = service.call(EnrichedFrame::from(request)).await?;
            Ok(enriched_frame)
        })
    }
}
