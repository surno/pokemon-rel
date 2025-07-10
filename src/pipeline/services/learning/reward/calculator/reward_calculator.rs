use crate::pipeline::services::learning::reward::multi_objective_reward::MultiObjectiveReward;
use crate::pipeline::types::{EnrichedFrame, GameAction};

pub trait RewardCalculator: Send + Sync {
    fn calculate_reward(
        &self,
        current_frame: &EnrichedFrame,
        action: GameAction,
        next_frame: Option<&EnrichedFrame>,
    ) -> f32;
}
