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

        // Overworld incentives/penalties:
        // - Strongly penalize standing still in overworld (no perceptual change)
        // - Slight reward for movement that changes the image
        // - Slight penalty for spamming A/Start/B in overworld with no change
        // Existing intro/menu transitions remain rewarded to encourage progress

        // Handle Intro transitions as before
        if current_scene == Scene::Intro
            && next_scene != Scene::Intro
            && next_scene != Scene::Unknown
        {
            return 1.0;
        }
        if next_scene != Scene::Unknown && next_scene != current_scene {
            return 0.5;
        }

        // If we lack a next frame, provide a tiny penalty to push progress
        let Some(nf) = next_frame else {
            return -0.01;
        };

        // Use simple state-based change detection instead of expensive perceptual hashing
        // This avoids costly image resizing and hashing operations
        let changed = current_scene != next_scene || current_frame.state != nf.state;

        // Overworld is our catch-all when not Battle/MainMenu/Intro
        let in_overworld = current_scene != Scene::Battle
            && current_scene != Scene::MainMenu
            && current_scene != Scene::Intro;

        if in_overworld {
            match _action {
                // Movement: reward only if it changes the image; otherwise light penalty
                GameAction::Up | GameAction::Down | GameAction::Left | GameAction::Right => {
                    if changed {
                        0.05
                    } else {
                        -0.1
                    }
                }
                // Interaction without visible effect: penalize to avoid talking when nothing changes
                GameAction::A => {
                    if changed {
                        0.02
                    } else {
                        -0.15
                    }
                }
                // Menu toggles in overworld: discourage
                GameAction::Start | GameAction::B => {
                    if changed {
                        -0.02
                    } else {
                        -0.1
                    }
                }
                // D-pad diagonals or others (if any) default to small penalty if no change
                _ => {
                    if changed {
                        0.0
                    } else {
                        -0.05
                    }
                }
            }
        } else {
            // Default gentle nudge towards progress when not in overworld
            if changed { 0.0 } else { -0.01 }
        }
    }
}
