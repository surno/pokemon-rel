use std::sync::{Arc, RwLock};

use crate::error::AppError;
use crate::pipeline::types::EnrichedFrame;
use crate::pipeline::services::learning::experience_collector::ExperienceCollector;
use crate::pipeline::services::learning::reward::processor::multi_objective_reward_processor::MultiObjectiveRewardProcessor;

pub struct AppController {
    reward_processor: MultiObjectiveRewardProcessor,
    experience_collector: Arc<RwLock<ExperienceCollector>>,
    frame_rx: Receiver<EnrichedFrame>,
}

impl AppController {
    pub fn new(frame_rx: Receiver<EnrichedFrame>) -> Self {
        Self {
            reward_processor: MultiObjectiveRewardProcessor::new(Box::new(NavigationRewardCalculator::default())),
            experience_collector: Arc::new(RwLock::new(ExperienceCollector)),
            frame_rx,
        }
    }

    pub async fn run(&mut self) -> Result<(), AppError> {
        loop {
            if let Some(frame) = self {
        }
    }
}

