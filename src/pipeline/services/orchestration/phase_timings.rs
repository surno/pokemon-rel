use crate::pipeline::services::orchestration::pipeline_phase::PipelinePhase;
use indexmap::IndexMap;
use std::time::{Duration, Instant};

/// Tracks timing information for phases and steps in the pipeline
#[derive(Debug, Clone, Default)]
pub struct PhaseTimings {
    /// Total duration per phase
    phase_durations: IndexMap<PipelinePhase, Duration>,
    /// Duration per step within each phase
    step_durations: IndexMap<(PipelinePhase, String), Duration>,
    /// Current phase entry times (for ongoing tracking)
    current_phase_starts: IndexMap<PipelinePhase, Instant>,
    /// Current step entry times
    current_step_starts: IndexMap<(PipelinePhase, String), Instant>,
}

impl PhaseTimings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record entry into a phase
    pub fn entry_phase(&mut self, phase: PipelinePhase) {
        self.current_phase_starts.insert(phase, Instant::now());
    }

    /// Record exit from a phase and accumulate duration
    pub fn exit_phase(&mut self, phase: PipelinePhase, duration: Duration) {
        if self.current_phase_starts.remove(&phase).is_some() {
            // Accumulate phase duration
            let entry = self.phase_durations.entry(phase).or_insert_with(|| Duration::from_secs(0));
            *entry += duration;
        } else {
            // First time seeing this phase
            self.phase_durations.insert(phase, duration);
        }
    }

    /// Record entry into a step
    pub fn entry_step(&mut self, phase: PipelinePhase, step_name: String) {
        self.current_step_starts.insert((phase, step_name), Instant::now());
    }

    /// Record exit from a step and accumulate duration
    pub fn exit_step(
        &mut self,
        phase: PipelinePhase,
        step_name: String,
        duration: Duration,
    ) {
        let key = (phase, step_name);
        if self.current_step_starts.remove(&key).is_some() {
            let entry = self.step_durations.entry(key).or_insert_with(|| Duration::from_secs(0));
            *entry += duration;
        } else {
            self.step_durations.insert(key, duration);
        }
    }

    /// Get total duration for a phase
    pub fn get_phase_duration(&self, phase: &PipelinePhase) -> Duration {
        self.phase_durations
            .get(phase)
            .copied()
            .unwrap_or_else(|| Duration::from_secs(0))
    }

    /// Get duration for a specific step in a phase
    pub fn get_step_duration(
        &self,
        phase: &PipelinePhase,
        step_name: &str,
    ) -> Duration {
        self.step_durations
            .get(&(phase.clone(), step_name.to_string()))
            .copied()
            .unwrap_or_else(|| Duration::from_secs(0))
    }

    /// Get all phase durations
    pub fn get_all_phase_durations(&self) -> &IndexMap<PipelinePhase, Duration> {
        &self.phase_durations
    }

    /// Get all step durations for a phase
    pub fn get_phase_step_durations(
        &self,
        phase: &PipelinePhase,
    ) -> IndexMap<String, Duration> {
        self.step_durations
            .iter()
            .filter_map(|((p, step_name), duration)| {
                if p == phase {
                    Some((step_name.clone(), *duration))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Reset all timings
    pub fn reset(&mut self) {
        self.phase_durations.clear();
        self.step_durations.clear();
        self.current_phase_starts.clear();
        self.current_step_starts.clear();
    }
}
