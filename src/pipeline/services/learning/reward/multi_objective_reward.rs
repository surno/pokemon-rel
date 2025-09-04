use crate::pipeline::types::{EnrichedFrame, GameAction};

#[derive(Clone, Debug)]
pub struct MultiObjectiveReward {
    pub navigation_reward: f32,
    pub battle_reward: f32,
}

impl MultiObjectiveReward {
    pub fn to_vector(&self) -> Vec<f32> {
        vec![self.navigation_reward, self.battle_reward]
    }

    pub fn normalize(&self) -> f32 {
        let v = self.to_vector();
        let sum: f32 = v.iter().copied().sum();
        sum / v.len() as f32
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
