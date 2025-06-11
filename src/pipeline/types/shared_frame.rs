use crate::pipeline::types::{EnrichedFrame, GameAction, RLPrediction, RawFrame};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SharedFrame {
    pub raw: Arc<RawFrame>,
    pub enriched: Option<Arc<EnrichedFrame>>,
    pub ml_prediction: Option<Arc<RLPrediction>>,
    pub game_action: Option<Arc<GameAction>>,
}

impl From<RawFrame> for SharedFrame {
    fn from(raw: RawFrame) -> Self {
        Self {
            raw: Arc::new(raw),
            enriched: None,
            ml_prediction: None,
            game_action: None,
        }
    }
}
