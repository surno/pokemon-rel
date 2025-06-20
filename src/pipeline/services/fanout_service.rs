use crate::error::AppError;
use crate::pipeline::{
    services::{MLPipelineService, preprocessing::FrameHashingService},
    types::{EnrichedFrame, GameAction, RawFrame},
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::broadcast;
use tower::Service;
use tracing::debug;

pub struct FanoutService {
    visualization_tx: broadcast::Sender<EnrichedFrame>,
    ml_service: MLPipelineService,
}

impl FanoutService {
    pub fn new(_frame_hashing_service: FrameHashingService) -> Self {
        let ml_service = MLPipelineService {};
        let (visualization_tx, _) = broadcast::channel(10);
        Self {
            visualization_tx,
            ml_service,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EnrichedFrame> {
        self.visualization_tx.subscribe()
    }
}

impl Service<RawFrame> for FanoutService {
    type Response = GameAction;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: RawFrame) -> Self::Future {
        todo!()
    }
}
