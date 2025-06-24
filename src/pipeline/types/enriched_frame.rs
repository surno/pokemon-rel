use chrono::Utc;
use image::DynamicImage;
use uuid::Uuid;

use crate::{
    intake::frame::frame::Frame,
    pipeline::types::{GameAction, State},
};

#[derive(Clone)]
pub struct EnrichedFrame {
    pub client: Uuid,
    pub image: DynamicImage,
    pub timestamp: i64,
    pub program: u16,
    pub id: Uuid,
    pub state: Option<State>,
    pub action: Option<GameAction>,
}

impl EnrichedFrame {
    pub fn new(client: Uuid, image: DynamicImage, program: u16) -> Self {
        Self {
            client,
            image,
            timestamp: Utc::now().timestamp_millis(),
            id: Uuid::new_v4(),
            state: None,
            action: None,
            program,
        }
    }
}
