use crate::pipeline::services::{
    action_service::ActionService, preprocessing::PreprocessingService, rl_service::RLService,
};
use crate::pipeline::types::{GameAction, RawFrame};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

#[derive(Debug, Clone)]
pub struct MLPipelineService {
    preprocessing_service: PreprocessingService,
    rl_service: RLService,
    action_service: ActionService,
}

impl MLPipelineService {
    pub fn new() -> Self {
        Self {
            preprocessing_service: PreprocessingService,
            rl_service: RLService,
            action_service: ActionService,
        }
    }
}

impl Service<RawFrame> for MLPipelineService {
    type Response = GameAction;
    type Error = crate::error::PipelineError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: RawFrame) -> Self::Future {
        let mut preprocessing_service = self.preprocessing_service.clone();
        let mut rl_service = self.rl_service.clone();
        let mut action_service = self.action_service.clone();

        Box::pin(async move {
            let enriched_frame = preprocessing_service.call(request).await?;
            let rl_prediction = rl_service.call(enriched_frame).await?;
            let game_action = action_service.call(rl_prediction).await?;
            Ok(game_action)
        })
    }
}
