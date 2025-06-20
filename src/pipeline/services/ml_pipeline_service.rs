use crate::error::AppError;
use crate::pipeline::types::RawFrame;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::TcpStream;
use tower::Service;

pub struct MLPipelineService {}

impl MLPipelineService {
    pub fn new() -> Self {
        Self {}
    }
}

impl Service<TcpStream> for MLPipelineService {
    type Response = RawFrame;
    type Error = AppError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        return Poll::Ready(Ok(()));
    }

    fn call(&mut self, _: TcpStream) -> Self::Future {
        Box::pin(async move { todo!() })
    }
}
