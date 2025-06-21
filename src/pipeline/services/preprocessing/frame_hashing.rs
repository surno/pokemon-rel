use crate::{
    error::AppError,
    pipeline::{EnrichedFrame, GameState},
};
use bloomfilter::Bloom;
use image::DynamicImage;
use imghash::{ImageHasher, perceptual::PerceptualHasher};
use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower::Service;

#[derive(Debug, Clone)]
pub struct FrameHashingBuilder {
    bloom_filters: HashMap<GameState, Bloom<String>>,
    capacity: usize,
    fp_rate: f64,
}

impl FrameHashingBuilder {
    pub fn new(capacity: usize, fp_rate: f64) -> Self {
        Self {
            bloom_filters: HashMap::new(),
            capacity,
            fp_rate,
        }
    }

    pub fn with_game_state(mut self, game_state: GameState, hashes: Vec<String>) -> Self {
        let mut bloom_filter = Bloom::new_for_fp_rate(self.capacity, self.fp_rate).unwrap();
        for hash in hashes {
            bloom_filter.set(&hash);
        }
        self.bloom_filters.insert(game_state, bloom_filter);
        self
    }

    pub fn build(self) -> FrameHashingService {
        FrameHashingService {
            bloom_filters: self.bloom_filters,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameHashingService {
    bloom_filters: HashMap<GameState, Bloom<String>>,
}

impl FrameHashingService {
    pub fn new(bloom_filters: HashMap<GameState, Bloom<String>>) -> Self {
        Self { bloom_filters }
    }

    fn detect_game_state(&self, frame: &DynamicImage) -> GameState {
        let hash = self.hash_frame(frame);
        self.bloom_filters
            .iter()
            .find(|(_, filter)| filter.check(&hash))
            .map(|(game_state, _)| *game_state)
            .unwrap_or(GameState::Unknown)
    }

    fn hash_frame(&self, frame: &DynamicImage) -> String {
        let hash = PerceptualHasher::default().hash_from_img(frame);
        hash.encode()
    }
}

impl Service<EnrichedFrame> for FrameHashingService {
    type Response = EnrichedFrame;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut enriched_frame: EnrichedFrame) -> Self::Future {
        let game_state = self.detect_game_state(&enriched_frame.raw.image);
        enriched_frame.game_state = Some(Arc::new(game_state));
        Box::pin(async move { Ok(enriched_frame) })
    }
}
