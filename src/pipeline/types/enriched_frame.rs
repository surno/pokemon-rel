use crate::pipeline::{
    GameState,
    types::{GameAction, RLPrediction, RawFrame},
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct EnrichedFrame {
    pub raw: Arc<RawFrame>,
    pub game_state: Option<Arc<GameState>>,
    pub ml_prediction: Option<Arc<RLPrediction>>,
    pub game_action: Option<Arc<GameAction>>,
}

impl From<RawFrame> for EnrichedFrame {
    fn from(raw: RawFrame) -> Self {
        Self {
            raw: Arc::new(raw),
            game_state: None,
            ml_prediction: None,
            game_action: None,
        }
    }
}
