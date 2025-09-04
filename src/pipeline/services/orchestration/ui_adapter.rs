/// UI Adapter for the new pipeline architecture
/// Provides backward-compatible interfaces for UI components that need access to pipeline stats
use crate::pipeline::services::{
    learning::smart_action_service::ActionDecision, orchestration::metrics::PerformanceStats,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

/// Backward-compatible stats structure for the UI
#[derive(Debug, Clone)]
pub struct UICompatibleStats {
    pub total_frames_processed: usize,
    pub total_decisions_made: usize,
    pub average_confidence: f32,
    pub last_decision_time: Option<Instant>,
    pub frames_per_sec: f32,
    pub decisions_per_sec: f32,
    pub total_actions_sent: usize,
    pub timing: UICompatibleTimingStats,
}

#[derive(Debug, Clone, Default)]
pub struct UICompatibleTimingStats {
    // EWMA timings
    pub analyze_situation_us: f32,
    pub hash_distance_us: f32,
    pub policy_inference_us: f32,
    pub macro_selection_us: f32,
    pub reward_processing_us: f32,
    pub experience_collection_us: f32,
    pub action_send_us: f32,
    pub total_frame_us: f32,

    // Last frame timings (for UI compatibility)
    pub last_analyze_situation_us: u64,
    pub last_hash_distance_us: u64,
    pub last_policy_inference_us: u64,
    pub last_macro_selection_us: u64,
    pub last_reward_processing_us: u64,
    pub last_experience_collection_us: u64,
    pub last_action_send_us: u64,
    pub last_total_frame_us: u64,

    // Max timings (for UI compatibility)
    pub max_analyze_situation_us: u64,
    pub max_hash_distance_us: u64,
    pub max_policy_inference_us: u64,
    pub max_macro_selection_us: u64,
    pub max_reward_processing_us: u64,
    pub max_experience_collection_us: u64,
    pub max_action_send_us: u64,
    pub max_total_frame_us: u64,
}

/// Backward-compatible debug snapshot for the UI
#[derive(Debug, Clone, Default)]
pub struct UICompatibleDebugSnapshot {
    pub last_client: Option<Uuid>,
    pub active_macro: Option<(crate::pipeline::MacroAction, u32)>,
    pub median_distance: Option<usize>,
}

/// Adapter that provides UI-compatible interfaces to the new pipeline architecture
pub struct UIPipelineAdapter {
    performance_stats: Arc<Mutex<PerformanceStats>>,
    decision_history: Arc<Mutex<std::collections::HashMap<Uuid, Vec<ActionDecision>>>>,
    debug_info: Arc<Mutex<crate::pipeline::services::orchestration::metrics::DebugInfo>>,
}

impl UIPipelineAdapter {
    pub fn new(
        performance_stats: Arc<Mutex<PerformanceStats>>,
        decision_history: Arc<Mutex<std::collections::HashMap<Uuid, Vec<ActionDecision>>>>,
        debug_info: Arc<Mutex<crate::pipeline::services::orchestration::metrics::DebugInfo>>,
    ) -> Self {
        Self {
            performance_stats,
            decision_history,
            debug_info,
        }
    }

    /// Get stats in the format expected by the UI
    pub fn get_stats_shared(&self) -> UICompatibleStats {
        let perf_stats = self.performance_stats.lock().unwrap().clone();
        tracing::debug!(
            "UI Adapter stats: frames={}, fps={:.1}",
            perf_stats.total_frames_processed,
            perf_stats.frames_per_second
        );

        UICompatibleStats {
            total_frames_processed: perf_stats.total_frames_processed,
            total_decisions_made: perf_stats.total_frames_processed, // Approximate
            average_confidence: 0.7,                                 // Default confidence
            last_decision_time: Some(perf_stats.last_fps_calculation),
            frames_per_sec: perf_stats.frames_per_second,
            decisions_per_sec: perf_stats.frames_per_second, // Approximate
            total_actions_sent: perf_stats.total_actions_sent,
            timing: UICompatibleTimingStats {
                // EWMA timings
                analyze_situation_us: perf_stats.avg_scene_analysis_us,
                hash_distance_us: perf_stats.avg_image_change_detection_us,
                policy_inference_us: perf_stats.avg_policy_inference_us,
                macro_selection_us: perf_stats.avg_macro_execution_us,
                reward_processing_us: perf_stats.avg_reward_processing_us,
                experience_collection_us: perf_stats.avg_experience_collection_us,
                action_send_us: perf_stats.avg_action_send_us,
                total_frame_us: perf_stats.average_frame_time_us,

                // Max timings
                max_analyze_situation_us: perf_stats.max_scene_analysis_us,
                max_hash_distance_us: perf_stats.max_image_change_detection_us,
                max_policy_inference_us: perf_stats.max_policy_inference_us,
                max_macro_selection_us: perf_stats.max_macro_execution_us,
                max_reward_processing_us: perf_stats.max_reward_processing_us,
                max_experience_collection_us: perf_stats.max_experience_collection_us,
                max_action_send_us: perf_stats.max_action_send_us,
                max_total_frame_us: 0, // This isn't tracked, using average

                // last_ timings are not available in the new architecture, using averages as fallback
                last_analyze_situation_us: perf_stats.avg_scene_analysis_us as u64,
                last_hash_distance_us: perf_stats.avg_image_change_detection_us as u64,
                last_policy_inference_us: perf_stats.avg_policy_inference_us as u64,
                last_macro_selection_us: perf_stats.avg_macro_execution_us as u64,
                last_reward_processing_us: perf_stats.avg_reward_processing_us as u64,
                last_experience_collection_us: perf_stats.avg_experience_collection_us as u64,
                last_action_send_us: perf_stats.avg_action_send_us as u64,
                last_total_frame_us: perf_stats.average_frame_time_us as u64,
            },
        }
    }

    /// Get debug snapshot in the format expected by the UI
    pub fn get_debug_snapshot(&self) -> UICompatibleDebugSnapshot {
        let debug_info = self.debug_info.lock().unwrap().clone();

        UICompatibleDebugSnapshot {
            last_client: debug_info.last_client,
            active_macro: None, // This would need to be populated from macro manager
            median_distance: None, // This would need to be populated from image change detector
        }
    }

    /// Get client decisions in the format expected by the UI
    pub fn get_client_decisions(&self, client_id: &Uuid) -> Vec<ActionDecision> {
        self.decision_history
            .lock()
            .unwrap()
            .get(client_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Add a decision to the history (called by the pipeline)
    pub fn add_client_decision(&self, client_id: Uuid, decision: ActionDecision) {
        let mut history = self.decision_history.lock().unwrap();
        let client_history = history.entry(client_id).or_insert_with(Vec::new);
        client_history.push(decision);

        // Keep only last 100 decisions per client
        if client_history.len() > 100 {
            client_history.remove(0);
        }
    }
}

impl Clone for UIPipelineAdapter {
    fn clone(&self) -> Self {
        Self {
            performance_stats: Arc::clone(&self.performance_stats),
            decision_history: Arc::clone(&self.decision_history),
            debug_info: Arc::clone(&self.debug_info),
        }
    }
}
