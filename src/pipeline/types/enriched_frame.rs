use crate::pipeline::types::{GameState, RawFrame};

#[derive(Debug, Clone)]
pub struct EnrichedFrame {
    pub raw_frame: RawFrame,
    pub game_state: GameState,
    pub features: Vec<f32>,
}
