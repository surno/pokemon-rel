use bytes::Bytes;
use image::{DynamicImage, RgbImage};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RawFrame {
    pub image: DynamicImage,
    pub timestamp: u64,
    pub id: Uuid,
}

impl RawFrame {
    pub fn new(width: u32, height: u32, pixels: Bytes) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let id = Uuid::new_v4();
        let image =
            DynamicImage::ImageRgb8(RgbImage::from_raw(width, height, pixels.to_vec()).unwrap());
        Self {
            image,
            timestamp,
            id,
        }
    }
}
