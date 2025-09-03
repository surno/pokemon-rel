use crate::error::AppError;
use crate::pipeline::services::learning::smart_action_service::SmartActionService;
use crate::pipeline::types::{EnrichedFrame, GameAction};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

pub struct ActionService;

impl ActionService {
    pub fn new() -> Self {
        Self
    }
}

impl Service<EnrichedFrame> for ActionService {
    type Response = GameAction;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), AppError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: EnrichedFrame) -> Self::Future {
        Box::pin(async move {
            // Create a new smart service instance for this call
            let mut smart_service = SmartActionService::new();

            // Get smart decision from the smart service
            let decision = smart_service.call(request).await?;

            // Return the chosen action
            Ok(decision.action)
        })
    }
}
