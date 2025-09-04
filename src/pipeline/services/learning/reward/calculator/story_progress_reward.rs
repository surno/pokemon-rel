use super::reward_calculator::RewardCalculator;
use crate::pipeline::types::{EnrichedFrame, GameAction, StoryProgress};

/// Reward calculator focused on story progression and badge collection
/// This addresses the critical flaw where the agent has no incentive to progress through the game
pub struct StoryProgressRewardCalculator {
    previous_story_progress: Option<StoryProgress>,
    previous_badges: u32,
    previous_pokedex_seen: u32,
    previous_pokedex_caught: u32,
}

impl Default for StoryProgressRewardCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl StoryProgressRewardCalculator {
    pub fn new() -> Self {
        Self {
            previous_story_progress: None,
            previous_badges: 0,
            previous_pokedex_seen: 0,
            previous_pokedex_caught: 0,
        }
    }

    /// Calculate reward for story progress changes
    fn story_progress_reward(&self, from: StoryProgress, to: StoryProgress) -> f32 {
        use StoryProgress::*;

        match (from, to) {
            // Early game progression - critical for getting started
            (GameStart, StarterObtained) => 5.0,
            (StarterObtained, FirstGym) => 10.0,

            // Gym progression - core game loop
            (FirstGym, SecondGym) => 10.0,
            (SecondGym, ThirdGym) => 10.0,
            (ThirdGym, FourthGym) => 10.0,
            (FourthGym, FifthGym) => 10.0,
            (FifthGym, SixthGym) => 10.0,
            (SixthGym, SeventhGym) => 10.0,
            (SeventhGym, EighthGym) => 10.0,

            // End game progression - higher rewards for final challenges
            (EighthGym, EliteFour) => 15.0,
            (EliteFour, Champion) => 20.0,
            (Champion, PostGame) => 15.0,

            // Any other forward progress gets a moderate reward
            _ => {
                // Check if this is forward progress by comparing enum discriminants
                let from_value = from as u8;
                let to_value = to as u8;

                if to_value > from_value {
                    5.0 // Forward progress not explicitly mapped
                } else {
                    0.0 // No progress or backwards (shouldn't happen normally)
                }
            }
        }
    }

    /// Calculate reward for badge increases
    fn badge_reward(&self, previous_badges: u32, current_badges: u32) -> f32 {
        if current_badges > previous_badges {
            let badges_gained = current_badges - previous_badges;
            badges_gained as f32 * 8.0 // 8.0 reward per badge
        } else {
            0.0
        }
    }

    /// Calculate reward for Pokédex progress
    fn pokedex_reward(
        &self,
        prev_seen: u32,
        curr_seen: u32,
        prev_caught: u32,
        curr_caught: u32,
    ) -> f32 {
        let mut reward = 0.0;

        // Reward for seeing new Pokémon
        if curr_seen > prev_seen {
            let new_seen = curr_seen - prev_seen;
            reward += new_seen as f32 * 0.5; // 0.5 per new species seen
        }

        // Higher reward for catching new Pokémon
        if curr_caught > prev_caught {
            let new_caught = curr_caught - prev_caught;
            reward += new_caught as f32 * 1.0; // 1.0 per new species caught
        }

        reward
    }

    /// Update internal state tracking for next frame comparison
    fn update_state(&mut self, frame: &EnrichedFrame) {
        if let Some(state) = &frame.state {
            self.previous_story_progress = Some(state.story_progress);
            self.previous_badges = state.badges_earned;
            self.previous_pokedex_seen = state.pokedex_seen;
            self.previous_pokedex_caught = state.pokedex_caught;
        }
    }
}

impl RewardCalculator for StoryProgressRewardCalculator {
    fn calculate_reward(
        &mut self,
        current_frame: &EnrichedFrame,
        _action: GameAction,
        _next_frame: Option<&EnrichedFrame>,
    ) -> f32 {
        let mut total_reward = 0.0;

        if let Some(current_state) = &current_frame.state {
            // Story progress reward
            if let Some(prev_progress) = &self.previous_story_progress {
                if current_state.story_progress != *prev_progress {
                    let story_reward =
                        self.story_progress_reward(*prev_progress, current_state.story_progress);
                    total_reward += story_reward;

                    // Log significant story progress for debugging
                    if story_reward > 0.0 {
                        tracing::info!(
                            "Story progress reward: {:?} -> {:?} = +{:.1}",
                            prev_progress,
                            current_state.story_progress,
                            story_reward
                        );
                    }
                }
            }

            // Badge reward
            if current_state.badges_earned > self.previous_badges {
                let badge_reward =
                    self.badge_reward(self.previous_badges, current_state.badges_earned);
                total_reward += badge_reward;

                tracing::info!(
                    "Badge progress reward: {} -> {} badges = +{:.1}",
                    self.previous_badges,
                    current_state.badges_earned,
                    badge_reward
                );
            }

            // Pokédex reward
            let pokedex_reward = self.pokedex_reward(
                self.previous_pokedex_seen,
                current_state.pokedex_seen,
                self.previous_pokedex_caught,
                current_state.pokedex_caught,
            );

            if pokedex_reward > 0.0 {
                total_reward += pokedex_reward;

                tracing::debug!(
                    "Pokédex progress reward: seen {}->{}, caught {}->{} = +{:.1}",
                    self.previous_pokedex_seen,
                    current_state.pokedex_seen,
                    self.previous_pokedex_caught,
                    current_state.pokedex_caught,
                    pokedex_reward
                );
            }

            // Update state for next comparison
            self.update_state(current_frame);
        }

        total_reward
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::types::{LocationType, Scene, State};

    fn create_test_frame(
        story_progress: StoryProgress,
        badges: u32,
        seen: u32,
        caught: u32,
    ) -> EnrichedFrame {
        use image::{DynamicImage, ImageBuffer, Rgb};
        use std::sync::Arc;
        use uuid::Uuid;

        let img: DynamicImage = DynamicImage::ImageRgb8(
            ImageBuffer::<Rgb<u8>, Vec<u8>>::from_pixel(16, 16, Rgb([1, 2, 3])),
        );

        EnrichedFrame {
            client: Uuid::new_v4(),
            image: Arc::new(img),
            timestamp: chrono::Utc::now().timestamp_millis(),
            program: 0,
            id: Uuid::new_v4(),
            action: None,
            color_analysis: None,
            state: Some(State {
                scene: Scene::Overworld,
                player_position: (0.0, 0.0),
                pokemon_count: 1,
                current_location: Some("Test Location".to_string()),
                location_type: LocationType::Town,
                pokemon_party: vec![],
                pokedex_seen: seen,
                pokedex_caught: caught,
                badges_earned: badges,
                story_progress,
                in_tall_grass: false,
                menu_cursor_position: None,
                battle_turn: None,
                last_encounter_steps: 0,
                encounter_chain: 0,
            }),
        }
    }

    #[test]
    fn test_story_progress_rewards() {
        let mut calculator = StoryProgressRewardCalculator::new();

        // First frame - establishes baseline
        let frame1 = create_test_frame(StoryProgress::GameStart, 0, 0, 0);
        let reward1 = calculator.calculate_reward(&frame1, GameAction::A, None);
        assert_eq!(reward1, 0.0); // No previous state to compare

        // Second frame - story progress
        let frame2 = create_test_frame(StoryProgress::StarterObtained, 0, 1, 1);
        let reward2 = calculator.calculate_reward(&frame2, GameAction::A, None);
        assert_eq!(reward2, 5.0 + 0.5 + 1.0); // Story + seen + caught

        // Third frame - gym progress
        let frame3 = create_test_frame(StoryProgress::FirstGym, 1, 3, 2);
        let reward3 = calculator.calculate_reward(&frame3, GameAction::A, None);
        assert_eq!(reward3, 10.0 + 8.0 + 1.0 + 1.0); // Story + badge + 2 seen + 1 caught
    }

    #[test]
    fn test_badge_rewards() {
        let mut calculator = StoryProgressRewardCalculator::new();

        let frame1 = create_test_frame(StoryProgress::FirstGym, 0, 0, 0);
        calculator.calculate_reward(&frame1, GameAction::A, None);

        let frame2 = create_test_frame(StoryProgress::FirstGym, 2, 0, 0);
        let reward = calculator.calculate_reward(&frame2, GameAction::A, None);
        assert_eq!(reward, 16.0); // 2 badges * 8.0 each
    }

    #[test]
    fn test_pokedex_rewards() {
        let mut calculator = StoryProgressRewardCalculator::new();

        let frame1 = create_test_frame(StoryProgress::GameStart, 0, 5, 3);
        calculator.calculate_reward(&frame1, GameAction::A, None);

        let frame2 = create_test_frame(StoryProgress::GameStart, 0, 8, 5);
        let reward = calculator.calculate_reward(&frame2, GameAction::A, None);
        assert_eq!(reward, 1.5 + 2.0); // 3 seen * 0.5 + 2 caught * 1.0
    }
}
