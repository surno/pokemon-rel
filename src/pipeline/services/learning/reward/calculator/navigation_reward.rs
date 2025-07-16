use crate::pipeline::services::learning::reward::multi_objective_reward::MultiObjectiveReward;
use crate::pipeline::types::{EnrichedFrame, GameAction, Scene};

use super::reward_calculator::RewardCalculator;

pub struct NavigationRewardCalculator {
    previous_scene: Scene,
    steps_in_same_scene: u32,
}

impl Default for NavigationRewardCalculator {
    fn default() -> Self {
        Self {
            previous_scene: Scene::Unknown,
            steps_in_same_scene: 0,
        }
    }
}

impl RewardCalculator for NavigationRewardCalculator {
    fn calculate_reward(
        &self,
        current_frame: &EnrichedFrame,
        action: GameAction,
        next_frame: Option<&EnrichedFrame>,
    ) -> f32 {
        let current_scene = current_frame
            .state
            .as_ref()
            .map_or(Scene::Unknown, |s| s.scene);
        let next_scene = next_frame.as_ref().map_or(Scene::Unknown, |f| {
            f.state.as_ref().map_or(Scene::Unknown, |s| s.scene)
        });
        0.0
    }
}
