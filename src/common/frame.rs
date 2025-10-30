use chrono::{DateTime, Utc};
use image::DynamicImage;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct Frame {
    client_id: Uuid,
    image: Arc<DynamicImage>,
    captured_at: DateTime<Utc>,
    frame_id: Uuid,
}

impl Frame {
    pub fn new(
        client_id: Uuid,
        image: DynamicImage,
        captured_at: DateTime<Utc>,
        frame_id: Uuid,
    ) -> Self {
        Self {
            client_id,
            image: Arc::new(image),
            captured_at,
            frame_id,
        }
    }

    pub fn get_client_id(&self) -> Uuid {
        self.client_id
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
        let f1 = Frame::new(Uuid::new_v4(), img, Utc::now(), Uuid::new_v4());
        let f2 = f1.clone();
        assert!(Arc::ptr_eq(&f1.image, &f2.image));
    }
}
