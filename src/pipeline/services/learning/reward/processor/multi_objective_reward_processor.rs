use std::collections::VecDeque;

use uuid::Uuid;

use crate::pipeline::services::learning::experience_collector::Experience;
use crate::pipeline::services::learning::reward::RewardProcessor;
use crate::pipeline::services::learning::reward::calculator::reward_calculator::RewardCalculator;
use crate::pipeline::services::learning::reward::calculator::{
    BattleRewardCalculator, StoryProgressRewardCalculator,
};
use crate::pipeline::services::learning::reward::multi_objective_reward::MultiObjectiveReward;
use crate::pipeline::types::{EnrichedFrame, GameAction, RLPrediction};

pub struct MultiObjectiveRewardProcessor {
    frame_buffer: VecDeque<EnrichedFrame>,
    action_buffer: VecDeque<GameAction>,
    prediction_buffer: VecDeque<RLPrediction>,

    navigation_reward_calculator: Box<dyn RewardCalculator>,
    battle_reward_calculator: Box<dyn RewardCalculator>,
    story_progress_calculator: Box<dyn RewardCalculator>,
}

impl MultiObjectiveRewardProcessor {
    pub fn new(navigation_reward_calculator: Box<dyn RewardCalculator>) -> Self {
        Self {
            frame_buffer: VecDeque::with_capacity(3),
            action_buffer: VecDeque::with_capacity(3),
            prediction_buffer: VecDeque::with_capacity(3),
            navigation_reward_calculator,
            battle_reward_calculator: Box::new(BattleRewardCalculator::default()),
            story_progress_calculator: Box::new(StoryProgressRewardCalculator::new()),
        }
    }

    fn update_buffers(
        &mut self,
        frame: &EnrichedFrame,
        action: GameAction,
        prediction: RLPrediction,
    ) {
        self.frame_buffer.push_back(frame.clone());
        self.action_buffer.push_back(action);
        self.prediction_buffer.push_back(prediction);

        while self.frame_buffer.len() > 3 {
            self.frame_buffer.pop_front();
            self.action_buffer.pop_front();
            self.prediction_buffer.pop_front();
        }
    }
}

impl RewardProcessor for MultiObjectiveRewardProcessor {
    fn process_frame(
        &mut self,
        frame: &EnrichedFrame,
        action: GameAction,
        prediction: RLPrediction,
    ) -> Option<Experience> {
        self.update_buffers(frame, action, prediction);

        if self.frame_buffer.len() < 3 {
            // We don't have enough history to calculate the reward
            return None;
        }

        let previous_frame = &self.frame_buffer[0];
        let current_frame = &self.frame_buffer[1];
        let next_frame = &self.frame_buffer[2];

        let processed_action = &self.action_buffer[1];
        let processed_prediction = &self.prediction_buffer[1];

        let nav_reward = self.navigation_reward_calculator.calculate_reward(
            current_frame,
            processed_action.clone(),
            Some(next_frame),
        );
        let battle_reward = self.battle_reward_calculator.calculate_reward(
            current_frame,
            processed_action.clone(),
            Some(next_frame),
        );
        let story_reward = self.story_progress_calculator.calculate_reward(
            current_frame,
            processed_action.clone(),
            Some(next_frame),
        );
        // Simplified stall/oscillation penalties - avoid expensive image hashing
        // Use basic scene/state changes instead of perceptual hashing
        let prev_scene = previous_frame.state.as_ref().map(|s| s.scene);
        let curr_scene = current_frame.state.as_ref().map(|s| s.scene);
        let next_scene = next_frame.state.as_ref().map(|s| s.scene);

        let scene_changed_pc = prev_scene != curr_scene;
        let scene_changed_cn = curr_scene != next_scene;

        let stall_penalty = if !scene_changed_pc && !scene_changed_cn {
            0.1 // Reduced penalty, less expensive to compute
        } else {
            0.0
        };
        let oscillation_penalty = 0.0; // Disable expensive oscillation detection

        let navigation_reward_total = nav_reward - stall_penalty - oscillation_penalty;
        let detailed_reward = MultiObjectiveReward {
            navigation_reward: navigation_reward_total,
            battle_reward,
            story_progress_reward: story_reward,
        };

        let normalized_reward = detailed_reward.normalize();

        Some(Experience {
            id: Uuid::new_v4(),
            episode_id: Uuid::new_v4(),
            next_frame: Some(next_frame.clone()),
            frame: current_frame.clone(),
            action: processed_action.clone(),
            prediction: processed_prediction.clone(),
            reward: normalized_reward,
            detailed_reward,
        })
    }
}
