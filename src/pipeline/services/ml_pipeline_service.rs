use crate::error::AppError;
use crate::pipeline::EnrichedFrame;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

pub struct MLPipelineService {}

impl MLPipelineService {
    pub fn new() -> Self {
        Self {}
    }
}

impl Service<EnrichedFrame> for MLPipelineService {
    type Response = EnrichedFrame;
    type Error = AppError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        return Poll::Ready(Ok(()));
    }

    fn call(&mut self, _: EnrichedFrame) -> Self::Future {
        Box::pin(async move { todo!() })
    }
}
