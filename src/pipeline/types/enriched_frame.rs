use image::DynamicImage;
use uuid::Uuid;

use crate::pipeline::types::{GameAction, State};

#[derive(Clone)]
pub struct EnrichedFrame {
    pub client: Uuid,
    pub image: DynamicImage,
    pub timestamp: u64,
    pub id: Uuid,
    pub state: Option<State>,
    pub action: Option<GameAction>,
}

impl EnrichedFrame {
    pub fn new(client: Uuid, image: DynamicImage, timestamp: u64, id: Uuid) -> Self {
        Self {
            client,
            image,
            timestamp,
            id,
            state: None,
            action: None,
        }
    }
}
