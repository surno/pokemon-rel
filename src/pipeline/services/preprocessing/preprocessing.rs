use crate::error::AppError;
use crate::pipeline::services::preprocessing::frame_hashing::FrameHashingService;
use crate::pipeline::types::{EnrichedFrame, GameState, RawFrame};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tower::Service;
use tracing::info;

#[derive(Debug, Clone)]
pub struct PreprocessingService {
    frame_hashing_service: FrameHashingService,
}

impl PreprocessingService {
    pub fn new(hashes: Vec<String>) -> Self {
        Self {
            frame_hashing_service: FrameHashingService::new(hashes),
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
        let is_intro = self.frame_hashing_service.is_frame_in_hashes(&request);
        info!("Frame is intro: {}", is_intro);

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
