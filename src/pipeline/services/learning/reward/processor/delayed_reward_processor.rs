use std::collections::VecDeque;

use crate::pipeline::services::learning::experience_collector::Experience;
use crate::pipeline::services::learning::reward::calculator::RewardCalculator;
use crate::pipeline::services::learning::reward::processor::reward_processor::RewardProcessor;
use crate::pipeline::types::{EnrichedFrame, GameAction, RLPrediction};
use uuid::Uuid;

pub struct DelayedRewardProcessor {
    frame_buffer: VecDeque<EnrichedFrame>,
    action_buffer: VecDeque<GameAction>,
    prediction_buffer: VecDeque<RLPrediction>,
    reward_calculator: Box<dyn RewardCalculator>,
}

impl DelayedRewardProcessor {
    pub fn new(reward_calculator: Box<dyn RewardCalculator>) -> Self {
        Self {
            frame_buffer: VecDeque::new(),
            action_buffer: VecDeque::new(),
            prediction_buffer: VecDeque::new(),
            reward_calculator,
        }
    }

    fn insert_frame(&mut self, frame: &EnrichedFrame) {
        self.frame_buffer.push_back(frame.clone());
        if self.frame_buffer.len() >= 3 {
            self.frame_buffer.pop_front();
        }
    }

    pub fn calculate_reward(&self) -> Option<Experience> {
        let previous_frame = &self.frame_buffer[0];
        let current_frame = &self.frame_buffer[1];
        let next_frame = &self.frame_buffer[2];

        if let (Some(action), Some(prediction)) =
            (self.action_buffer.front(), self.prediction_buffer.front())
        {
            let reward = self.reward_calculator.calculate_reward(
                previous_frame,
                action.clone(),
                Some(next_frame),
            );

            return Some(Experience {
                id: Uuid::new_v4(),
                reward,
                action: action.clone(),
                episode_id: Uuid::new_v4(),
                prediction: prediction.clone(),
                frame: current_frame.clone(),
                next_frame: Some(next_frame.clone()),
            });
        }
        None
    }
}

impl RewardProcessor for DelayedRewardProcessor {
    fn process_frame(&mut self, frame: &EnrichedFrame) -> Option<Experience> {
        self.insert_frame(frame);

        if self.frame_buffer.len() == 3 {
            self.calculate_reward()
        } else {
            None
        }
    }
}
