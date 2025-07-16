use crate::pipeline::types::{EnrichedFrame, GameAction};

#[derive(Clone, Debug)]
pub struct MultiObjectiveReward {
    pub navigation_reward: f32,
}

impl MultiObjectiveReward {
    pub fn to_vector(&self) -> Vec<f32> {
        vec![self.navigation_reward]
    }

    pub fn normalize(&self) -> f32 {
        self.navigation_reward / self.to_vector().len() as f32
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
