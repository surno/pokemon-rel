use crate::error::AppError;
use crate::pipeline::services::preprocessing::frame_hashing::FrameHashingService;
use crate::pipeline::types::{EnrichedFrame, GameState, GameStateData, RawFrame};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::info;

use tower::Service;

#[derive(Debug, Clone)]
pub struct PreprocessingService {
    frame_hashing_service: FrameHashingService,
}

impl PreprocessingService {
    pub fn new(frame_hashing_service: FrameHashingService) -> Self {
        Self {
            frame_hashing_service,
        }
    }
}

impl Service<RawFrame> for PreprocessingService {
    type Response = EnrichedFrame;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: RawFrame) -> Self::Future {
        let game_state = self.frame_hashing_service.detect_game_state(&request);
        info!("Game state: {:?}", game_state);

        Box::pin(async move {
            let game_state = GameStateData {
                game_state,
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
