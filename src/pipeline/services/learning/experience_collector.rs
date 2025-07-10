use std::collections::VecDeque;

use rand::prelude::IteratorRandom;
use tokio::sync::mpsc;
use uuid::Uuid as UUid;

use crate::pipeline::types::{EnrichedFrame, GameAction, RLPrediction};

#[derive(Clone)]
pub struct Experience {
    id: UUid,
    frame: EnrichedFrame,
    action: GameAction,
    reward: f32,
    prediction: RLPrediction,
    episode_id: UUid,
}

pub struct ExperienceBuffer {
    experiences: VecDeque<Experience>,
    max_size: usize,
    current_episode_id: UUid,
}

impl ExperienceBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            experiences: VecDeque::new(),
            max_size,
            current_episode_id: UUid::new_v4(),
        }
    }

    pub fn add_experience(&mut self, experience: Experience) {
        self.experiences.push_back(experience);
        if self.experiences.len() > self.max_size {
            self.experiences.pop_front();
        }
    }

    pub fn start_new_episode(&mut self) {
        self.current_episode_id = UUid::new_v4();
    }

    pub fn get_recent_experiences(&self, n: usize) -> Vec<Experience> {
        self.experiences
            .iter()
            .rev()
            .take(n)
            .cloned()
            .collect::<Vec<Experience>>()
    }

    pub fn get_training_batch(&self, batch_size: usize) -> Vec<Experience> {
        let mut rng = rand::rng();

        self.experiences
            .iter()
            .cloned()
            .collect::<Vec<Experience>>()
            .into_iter()
            .choose_multiple(&mut rng, batch_size)
    }
}

pub struct ExperienceCollector {
    buffer: ExperienceBuffer,
    // reward_calculator: ,
    training_tx: mpsc::Sender<Vec<Experience>>,
}

impl ExperienceCollector {
    pub fn new(buffer: ExperienceBuffer, training_tx: mpsc::Sender<Vec<Experience>>) -> Self {
        Self {
            buffer,
            training_tx,
        }
    }
}
