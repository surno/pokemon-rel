use crate::error::RLServiceError;
use crate::pipeline::types::{EnrichedFrame, RLPrediction};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

#[derive(Debug, Clone)]
pub struct RLService;

impl Service<EnrichedFrame> for RLService {
    type Response = RLPrediction;
    type Error = RLServiceError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _request: EnrichedFrame) -> Self::Future {
        Box::pin(async move {
            let prediction = RLPrediction {
                action_probabilities: vec![0.0; 12],
                value_estimate: 0.0,
                confidence: 0.0,
            };
            Ok(prediction)
        })
    }
}
