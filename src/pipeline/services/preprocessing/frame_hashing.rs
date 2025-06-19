use crate::pipeline::{GameState, types::RawFrame};
use bloomfilter::Bloom;
use image::{DynamicImage, ImageBuffer};
use imghash::{ImageHasher, perceptual::PerceptualHasher};
use std::collections::HashMap;
use tracing::warn;

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
    pub fn detect_game_state(&self, frame: &RawFrame) -> GameState {
        let hash = self.hash_frame(frame);
        self.bloom_filters
            .iter()
            .find(|(_, filter)| filter.check(&hash))
            .map(|(game_state, _)| *game_state)
            .unwrap_or(GameState::Unknown)
    }

    fn hash_frame(&self, frame: &RawFrame) -> String {
        let image = match ImageBuffer::from_raw(frame.width, frame.height, frame.pixels.clone()) {
            Some(image) => image,
            None => {
                warn!("Failed to convert frame to image");
                return String::new();
            }
        };

        let image = DynamicImage::ImageRgb8(image);
        let hash = PerceptualHasher::default().hash_from_img(&image);
        hash.encode()
    }
}
