use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::pipeline::{
    services::MLPipelineService,
    types::{GameAction, RawFrame, SharedFrame},
};
use tokio::sync::broadcast;
use tower::Service;

#[derive(Debug, Clone)]
pub struct FanoutService {
    visualization_tx: broadcast::Sender<SharedFrame>,
    ml_service: MLPipelineService,
}

impl FanoutService {
    pub fn new(visualization_capacity: usize) -> (Self, broadcast::Receiver<SharedFrame>) {
        let (visualization_tx, visualization_rx) = broadcast::channel(visualization_capacity);
        let ml_service = MLPipelineService::new();
        (
            Self {
                visualization_tx,
                ml_service,
            },
            visualization_rx,
        )
    }
}

impl Service<RawFrame> for FanoutService {
    type Response = GameAction;
    type Error = crate::error::PipelineError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: RawFrame) -> Self::Future {
        let shared_frame = SharedFrame::from(request.clone());

        let _ = self.visualization_tx.send(shared_frame.clone());

        let ml_future = self.ml_service.call(request.clone());

        Box::pin(ml_future)
    }
}
