use crate::error::AppError;
use crate::pipeline::services::preprocessing::frame_hashing;
use crate::pipeline::types::{EnrichedFrame, GameState, RawFrame};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tower::Service;
use tracing::info;

#[derive(Debug, Clone)]
pub struct PreprocessingService;

impl Service<RawFrame> for PreprocessingService {
    type Response = EnrichedFrame;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: RawFrame) -> Self::Future {
        let frame_hash = frame_hashing::get_frame_hash(&request);

        Box::pin(async move {
            let game_state = GameState {
                player_position: (0.0, 0.0),
                pokemon_count: 0,
            };
            let features = vec![];

            Ok(EnrichedFrame {
                raw_frame: request,
                game_state,
                features,
            })
        })
    }
}
