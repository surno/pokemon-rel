use crate::pipeline::types::{EnrichedFrame, GameAction, Scene};

use super::reward_calculator::RewardCalculator;

pub struct BattleRewardCalculator;

impl Default for BattleRewardCalculator {
    fn default() -> Self {
        Self
    }
}

impl RewardCalculator for BattleRewardCalculator {
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

        // Simple heuristic for battles:
        // - Reward entering Battle from a non-Battle scene
        // - Small positive reward for staying in Battle (encourage continuing battle actions)
        // - Reward exiting Battle to a non-Battle scene (battle concluded)
        // - Small negative otherwise
        match (current_scene, next_scene) {
            (Scene::Battle, Scene::Battle) => 0.1,      // sustaining battle
            (s, Scene::Battle) if s != Scene::Battle => 0.5, // entered battle
            (Scene::Battle, s) if s != Scene::Battle => 1.0, // battle concluded
            _ => -0.01,
        }
    }
}


