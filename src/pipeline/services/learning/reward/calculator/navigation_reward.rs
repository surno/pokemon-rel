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
        _action: GameAction,
        next_frame: Option<&EnrichedFrame>,
    ) -> f32 {
        let current_scene = current_frame
            .state
            .as_ref()
            .map_or(Scene::Unknown, |s| s.scene);
        let next_scene = next_frame.as_ref().map_or(Scene::Unknown, |f| {
            f.state.as_ref().map_or(Scene::Unknown, |s| s.scene)
        });

        // Simple heuristic:
        // - Reward positively when transitioning from Intro to any other scene
        // - Reward small positive when moving between distinct non-Unknown scenes
        // - Small penalty otherwise to encourage progress
        if current_scene == Scene::Intro
            && next_scene != Scene::Intro
            && next_scene != Scene::Unknown
        {
            1.0
        } else if next_scene != Scene::Unknown && next_scene != current_scene {
            0.5
        } else {
            -0.01
        }
    }
}
