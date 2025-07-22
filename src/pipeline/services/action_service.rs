use crate::error::AppError;
use crate::pipeline::types::{EnrichedFrame, GameAction};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

#[derive(Debug, Clone)]
pub struct ActionService;

impl Service<EnrichedFrame> for ActionService {
    type Response = GameAction;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _request: EnrichedFrame) -> Self::Future {
        Box::pin(async move {
            // TODO: Implement action selection logic
            // TODO: Implement action execution logic
            // Verify that the action is valid
            Ok(GameAction::A) // dummy action
        })
    }
}
