use crate::error::AppError;
use crate::pipeline::EnrichedFrame;
use crate::pipeline::types::{GameAction, RawFrame};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

pub struct MLPipelineService {}

impl Service<RawFrame> for MLPipelineService {
    type Response = GameAction;
    type Error = AppError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        return Poll::Ready(Ok(()));
    }

    fn call(&mut self, request: RawFrame) -> Self::Future {
        todo!()
    }
}
