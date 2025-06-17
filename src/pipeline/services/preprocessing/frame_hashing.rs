use crate::pipeline::types::RawFrame;
use bloomfilter::Bloom;
use image::{DynamicImage, ImageBuffer};
use imghash::{ImageHasher, perceptual::PerceptualHasher};
use tracing::warn;

#[derive(Debug, Clone)]
pub struct FrameHashingService {
    bloom_filter: Bloom<String>,
}

impl FrameHashingService {
    pub fn new(hashes: Vec<String>) -> Self {
        let mut bloom_filter: Bloom<String> = Bloom::new_for_fp_rate(1000, 0.1).unwrap();
        for hash in hashes {
            bloom_filter.set(&hash);
        }

        Self { bloom_filter }
    }

    pub fn is_frame_in_hashes(&self, frame: &RawFrame) -> bool {
        let hash = self.hash_frame(frame);
        self.bloom_filter.check(&hash)
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
