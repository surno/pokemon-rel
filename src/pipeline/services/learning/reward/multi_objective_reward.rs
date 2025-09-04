use crate::pipeline::types::{EnrichedFrame, GameAction};

#[derive(Clone, Debug)]
pub struct MultiObjectiveReward {
    pub navigation_reward: f32,
    pub battle_reward: f32,
    pub story_progress_reward: f32,
}

impl MultiObjectiveReward {
    pub fn to_vector(&self) -> Vec<f32> {
        vec![
            self.navigation_reward,
            self.battle_reward,
            self.story_progress_reward,
        ]
    }

    pub fn normalize(&self) -> f32 {
        // Weighted sum with story progress having higher weight
        // Story progress should dominate the reward signal
        let weighted_sum = self.navigation_reward * 0.2
            + self.battle_reward * 0.3
            + self.story_progress_reward * 0.5;
        weighted_sum
    }
}

pub trait MultiObjectiveRewardCalculator: Send + Sync {
    fn calculate_reward(
        &self,
        current_frame: &EnrichedFrame,
        action: GameAction,
        next_frame: Option<&EnrichedFrame>,
    ) -> MultiObjectiveReward;
}
