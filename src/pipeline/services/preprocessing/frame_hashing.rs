use crate::pipeline::types::RawFrame;
use image::{DynamicImage, ImageBuffer};
use imghash::{ImageHasher, perceptual::PerceptualHasher};
use tracing::warn;

pub fn get_frame_hash(frame: &RawFrame) -> String {
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
