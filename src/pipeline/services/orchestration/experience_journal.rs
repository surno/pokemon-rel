use crate::pipeline::services::learning::experience_collector::Experience;
use crate::pipeline::types::{EnrichedFrame, GameAction, RLPrediction};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json;
use uuid::Uuid;

/// Structured journal entry for experience tracking
#[derive(Debug, Clone, Serialize)]
pub struct ExperienceJournalEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub client_id: Uuid,
    pub frame_id: Uuid,
    pub action: GameAction,
    pub reward: f32,
    pub prediction: RLPrediction,
    pub episode_id: Uuid,
    pub phase_durations: PhaseDurations,
    pub metadata: serde_json::Value,
}

/// Timing information for different pipeline phases
#[derive(Debug, Clone, Serialize, Default)]
pub struct PhaseDurations {
    pub analysis_us: Option<u64>,
    pub learning_us: Option<u64>,
    pub decision_us: Option<u64>,
    pub execution_us: Option<u64>,
    pub journaling_us: Option<u64>,
    pub total_us: u64,
}

/// Journal writer for structured experience logging
pub trait ExperienceJournalWriter: Send + Sync {
    fn write_entry(&mut self, entry: ExperienceJournalEntry) -> Result<(), crate::error::AppError>;
    fn flush(&mut self) -> Result<(), crate::error::AppError>;
}

/// In-memory journal writer (for testing and development)
pub struct InMemoryJournalWriter {
    entries: Vec<ExperienceJournalEntry>,
    max_entries: usize,
}

impl InMemoryJournalWriter {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_entries.min(1000)),
            max_entries,
        }
    }

    pub fn get_entries(&self) -> &[ExperienceJournalEntry] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl ExperienceJournalWriter for InMemoryJournalWriter {
    fn write_entry(&mut self, entry: ExperienceJournalEntry) -> Result<(), crate::error::AppError> {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), crate::error::AppError> {
        // In-memory writer doesn't need to flush
        Ok(())
    }
}

/// Builder for creating experience journal entries
pub struct ExperienceJournalEntryBuilder {
    experience: Experience,
    phase_durations: PhaseDurations,
    metadata: serde_json::Map<String, serde_json::Value>,
}

impl ExperienceJournalEntryBuilder {
    pub fn from_experience(experience: Experience) -> Self {
        Self {
            experience,
            phase_durations: PhaseDurations::default(),
            metadata: serde_json::Map::new(),
        }
    }

    pub fn with_phase_durations(mut self, durations: PhaseDurations) -> Self {
        self.phase_durations = durations;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn build(self) -> ExperienceJournalEntry {
        ExperienceJournalEntry {
            id: self.experience.id,
            timestamp: Utc::now(),
            client_id: self.experience.frame.client,
            frame_id: self.experience.frame.id,
            action: self.experience.action,
            reward: self.experience.reward,
            prediction: self.experience.prediction,
            episode_id: self.experience.episode_id,
            phase_durations: self.phase_durations,
            metadata: serde_json::Value::Object(self.metadata),
        }
    }
}
