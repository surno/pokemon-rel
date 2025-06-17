use crate::pipeline::types::{GameStateData, RawFrame};

#[derive(Debug, Clone)]
pub struct EnrichedFrame {
    pub raw_frame: RawFrame,
    pub game_state: GameStateData,
    pub features: Vec<f32>,
}
