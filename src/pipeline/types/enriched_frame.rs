use crate::pipeline::types::{GameAction, RLPrediction, RawFrame, State};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct EnrichedFrame {
    pub raw: Arc<RawFrame>,
    pub state: Option<State>,
    pub ml_prediction: Option<Arc<RLPrediction>>,
    pub game_action: Option<Arc<GameAction>>,
}

impl From<RawFrame> for EnrichedFrame {
    fn from(raw: RawFrame) -> Self {
        Self {
            raw: Arc::new(raw),
            state: None,
            ml_prediction: None,
            game_action: None,
        }
    }
}
