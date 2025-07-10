use crate::pipeline::types::{EnrichedFrame, GameAction};

pub trait RewardCalculator: Send + Sync {
    fn calculate_reward(
        current_frame: &EnrichedFrame,
        action: GameAction,
        next_frame: Option<&EnrichedFrame>,
    ) -> f32;
}
