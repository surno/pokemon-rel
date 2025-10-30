use std::collections::{HashMap, VecDeque};

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

/// Experience buffer using industry-standard data structures:
/// - VecDeque for efficient FIFO operations (O(1) push/pop)
/// - HashMap for fast episode-based lookups (O(1) average case)
pub struct ExperienceBuffer {
    /// Main experience storage (preserves temporal order)
    pub experiences: VecDeque<Experience>,
    /// Fast lookup: episode_id -> Vec<indices> into experiences
    /// Uses HashMap for O(1) episode lookups
    episode_index: HashMap<UUid, Vec<usize>>,
    max_size: usize,
    current_episode_id: UUid,
    /// Track the current offset for index calculation when VecDeque wraps
    start_index_offset: usize,
}

impl ExperienceBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            experiences: VecDeque::new(),
            episode_index: HashMap::new(),
            max_size,
            current_episode_id: UUid::new_v4(),
            start_index_offset: 0,
        }
    }

    /// Add an experience to the buffer
    /// Maintains both VecDeque (temporal order) and HashMap (episode indexing)
    pub fn add_experience(&mut self, experience: Experience) {
        let episode_id = experience.episode_id;
        let current_index = self.experiences.len() + self.start_index_offset;
        
        // Update episode index
        self.episode_index
            .entry(episode_id)
            .or_insert_with(Vec::new)
            .push(current_index);
        
        // Add to buffer
        self.experiences.push_back(experience);
        
        // Maintain max size (FIFO eviction)
        if self.experiences.len() > self.max_size {
            if let Some(removed) = self.experiences.pop_front() {
                // Remove from episode index
                if let Some(indices) = self.episode_index.get_mut(&removed.episode_id) {
                    indices.retain(|&i| i != self.start_index_offset);
                    if indices.is_empty() {
                        self.episode_index.remove(&removed.episode_id);
                    }
                }
                self.start_index_offset += 1;
            }
        }
    }

    pub fn start_new_episode(&mut self) {
        self.current_episode_id = UUid::new_v4();
    }
    
    /// Get experiences for a specific episode (O(1) lookup + O(k) where k = episode size)
    pub fn get_episode_experiences(&self, episode_id: &UUid) -> Vec<Experience> {
        if let Some(indices) = self.episode_index.get(episode_id) {
            indices
                .iter()
                .filter_map(|&idx| {
                    let adjusted_idx = idx.saturating_sub(self.start_index_offset);
                    self.experiences.get(adjusted_idx).cloned()
                })
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get experiences for the current episode
    pub fn get_current_episode_experiences(&self) -> Vec<Experience> {
        self.get_episode_experiences(&self.current_episode_id)
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

        if self.should_send_training_batch() {
            let batch = self.buffer.get_training_batch(100);
            if let Err(e) = self.training_tx.try_send(batch) {
                info!("Training batch not sent (channel not ready/closed): {}", e);
            }
        }
    }

    fn should_send_training_batch(&self) -> bool {
        let buffer_size = self.buffer.experiences.len();

        // Rudimentary batching logic, where we send a batch of a few experiences every 16 experiences
        buffer_size >= 32 && buffer_size % 16 == 0
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
