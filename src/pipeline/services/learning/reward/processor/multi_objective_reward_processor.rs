use std::collections::VecDeque;

use uuid::Uuid;

use crate::pipeline::services::learning::experience_collector::Experience;
use crate::pipeline::services::learning::reward::RewardProcessor;
use crate::pipeline::services::learning::reward::calculator::BattleRewardCalculator;
use crate::pipeline::services::learning::reward::calculator::reward_calculator::RewardCalculator;
use crate::pipeline::services::learning::reward::multi_objective_reward::MultiObjectiveReward;
use crate::pipeline::types::{EnrichedFrame, GameAction, RLPrediction};
use image::imageops::FilterType;
use imghash::{ImageHasher, perceptual::PerceptualHasher};

pub struct MultiObjectiveRewardProcessor {
    frame_buffer: VecDeque<EnrichedFrame>,
    action_buffer: VecDeque<GameAction>,
    prediction_buffer: VecDeque<RLPrediction>,

    navigation_reward_calculator: Box<dyn RewardCalculator>,
    battle_reward_calculator: Box<dyn RewardCalculator>,
}

impl MultiObjectiveRewardProcessor {
    pub fn new(navigation_reward_calculator: Box<dyn RewardCalculator>) -> Self {
        Self {
            frame_buffer: VecDeque::with_capacity(3),
            action_buffer: VecDeque::with_capacity(3),
            prediction_buffer: VecDeque::with_capacity(3),
            navigation_reward_calculator,
            battle_reward_calculator: Box::new(BattleRewardCalculator::default()),
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
        // Overworld stall/oscillation penalties using 3-frame context
        let hasher = PerceptualHasher::default();
        let small_prev = previous_frame.image.resize(128, 128, FilterType::Nearest);
        let small_curr = current_frame.image.resize(128, 128, FilterType::Nearest);
        let small_next = next_frame.image.resize(128, 128, FilterType::Nearest);
        let h_prev = hasher.hash_from_img(&small_prev);
        let h_curr = hasher.hash_from_img(&small_curr);
        let h_next = hasher.hash_from_img(&small_next);
        let d_pc = h_prev.distance(&h_curr).unwrap_or(0);
        let d_cn = h_curr.distance(&h_next).unwrap_or(0);
        let d_pn = h_prev.distance(&h_next).unwrap_or(0);
        let changed_pc = d_pc > 5;
        let changed_cn = d_cn > 5;
        let changed_pn = d_pn > 5;
        let stall_penalty = if !changed_pc && !changed_cn && !changed_pn {
            0.3
        } else {
            0.0
        };
        let oscillation_penalty = if !changed_pn && changed_pc && changed_cn {
            0.2
        } else {
            0.0
        };

        let navigation_reward_total = nav_reward - stall_penalty - oscillation_penalty;
        let detailed_reward = MultiObjectiveReward {
            navigation_reward: navigation_reward_total,
            battle_reward,
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
