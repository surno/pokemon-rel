use crate::error::AppError;
use crate::pipeline::{
    services::{MLPipelineService, preprocessing::SceneAnnotationService},
    types::{EnrichedFrame, GameAction, RawFrame},
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::broadcast;
use tower::Service;

#[derive(Clone)]
pub struct FramePublishingService {
    visualization_tx: broadcast::Sender<EnrichedFrame>,
}

impl FramePublishingService {
    pub fn new() -> (Self, broadcast::Receiver<EnrichedFrame>) {
        let (visualization_tx, visualization_rx) = broadcast::channel(10);
        (Self { visualization_tx }, visualization_rx)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EnrichedFrame> {
        self.visualization_tx.subscribe()
    }
}

impl Service<EnrichedFrame> for FramePublishingService {
    type Response = EnrichedFrame;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, enriched_frame: EnrichedFrame) -> Self::Future {
        let _ = self.visualization_tx.send(enriched_frame.clone());
        Box::pin(async move { Ok(enriched_frame) })
    }
}
