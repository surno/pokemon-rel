use chrono::Utc;
use image::DynamicImage;
use std::sync::Arc;
use uuid::Uuid;

use crate::pipeline::services::image::color_analysis_service::ColorAnalysis;
use crate::pipeline::types::{GameAction, State};

#[derive(Clone)]
pub struct EnrichedFrame {
    pub client: Uuid,
    pub image: Arc<DynamicImage>,
    pub timestamp: i64,
    pub program: u16,
    pub id: Uuid,
    pub state: Option<State>,
    pub action: Option<GameAction>,
    pub color_analysis: Option<ColorAnalysis>,
}

impl EnrichedFrame {
    pub fn new(client: Uuid, image: DynamicImage, program: u16) -> Self {
        Self {
            client,
            image: Arc::new(image),
            timestamp: Utc::now().timestamp_millis(),
            id: Uuid::new_v4(),
            state: None,
            action: None,
            color_analysis: None,
            program,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    #[test]
    fn cloning_frame_shares_image_buffer() {
        let img: DynamicImage = DynamicImage::ImageRgb8(
            ImageBuffer::<Rgb<u8>, Vec<u8>>::from_pixel(16, 16, Rgb([1, 2, 3])),
        );
        let f1 = EnrichedFrame::new(Uuid::new_v4(), img, 0);
        let f2 = f1.clone();
        assert!(Arc::ptr_eq(&f1.image, &f2.image));
    }
}
