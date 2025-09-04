use crate::pipeline::services::learning::experience_collector::Experience;
use crate::pipeline::types::{EnrichedFrame, GameAction, RLPrediction};

pub trait RewardProcessor: Send + Sync {
    fn process_frame(
        &mut self,
        frame: &EnrichedFrame,
        action: GameAction,
        prediction: RLPrediction,
    ) -> Option<Experience>;
}
