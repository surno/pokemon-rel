use std::collections::VecDeque;

use rand::prelude::IteratorRandom;
use tokio::sync::mpsc;
use tracing::info;
use uuid::Uuid as UUid;

use crate::pipeline::{
    services::learning::reward::multi_objective_reward::MultiObjectiveReward,
    types::{EnrichedFrame, GameAction, RLPrediction},
};

#[derive(Clone)]
pub struct Experience {
    pub id: UUid,
    pub reward: f32,
    pub action: GameAction,
    pub episode_id: UUid,
    pub prediction: RLPrediction,
    pub next_frame: Option<EnrichedFrame>,
    pub frame: EnrichedFrame,
    pub detailed_reward: MultiObjectiveReward,
}

pub struct ExperienceBuffer {
    pub experiences: VecDeque<Experience>,
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

    pub fn average_reward(&self) -> f32 {
        if self.experiences.is_empty() {
            return 0.0;
        }

        let total: f32 = self.experiences.iter().map(|e| e.reward).sum();
        total / self.experiences.len() as f32
    }
}

pub struct ExperienceCollector {
    pub buffer: ExperienceBuffer,
    pub training_tx: mpsc::Sender<Vec<Experience>>,

    total_experience_count: usize,
    total_episode_count: usize,
}

impl ExperienceCollector {
    pub fn new(max_size: usize, training_tx: mpsc::Sender<Vec<Experience>>) -> Self {
        Self {
            buffer: ExperienceBuffer::new(max_size),
            training_tx,
            total_experience_count: 0,
            total_episode_count: 0,
        }
    }

    pub async fn collect_experience(&mut self, experience: Experience) {
        self.total_experience_count += 1;
        self.buffer.add_experience(experience);

        if self.total_experience_count % 100 == 0 {
            info!(
                "Stats: Total = {} Buffer = {}, Avg Reward = {}",
                self.total_experience_count,
                self.buffer.experiences.len(),
                self.buffer.average_reward(),
            );
        }
    }

    pub async fn start_new_episode(&mut self) {
        self.buffer.start_new_episode();
        self.total_episode_count += 1;
    }

    pub fn get_stats(&self) -> ExperienceStats {
        ExperienceStats {
            total_experience_count: self.total_experience_count,
            total_episode_count: self.total_episode_count,
            buffer_size: self.buffer.experiences.len(),
            average_reward: self.buffer.average_reward(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExperienceStats {
    pub total_experience_count: usize,
    pub total_episode_count: usize,
    pub buffer_size: usize,
    pub average_reward: f32,
}
