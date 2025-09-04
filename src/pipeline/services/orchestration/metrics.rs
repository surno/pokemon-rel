use super::frame_context::{FrameMetrics, ProcessingStepType};
use crate::pipeline::GameAction;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

/// Observer pattern for metrics collection
pub trait MetricsObserver: Send + Sync {
    fn on_frame_processed(&mut self, client_id: Uuid, metrics: &FrameMetrics);
    fn on_action_sent(&mut self, client_id: Uuid, action: GameAction);
    fn on_processing_step(&mut self, client_id: Uuid, step: ProcessingStepType, duration_us: u64);
}

/// Collects and manages multiple metrics observers
pub struct MetricsCollector {
    observers: Vec<Box<dyn MetricsObserver>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
        }
    }

    pub fn add_observer(mut self, observer: Box<dyn MetricsObserver>) -> Self {
        self.observers.push(observer);
        self
    }

    pub fn notify_frame_processed(&mut self, client_id: Uuid, metrics: &FrameMetrics) {
        for observer in &mut self.observers {
            observer.on_frame_processed(client_id, metrics);
        }
    }

    pub fn notify_action_sent(&mut self, client_id: Uuid, action: GameAction) {
        for observer in &mut self.observers {
            observer.on_action_sent(client_id, action);
        }
    }

    pub fn notify_processing_step(
        &mut self,
        client_id: Uuid,
        step: ProcessingStepType,
        duration_us: u64,
    ) {
        for observer in &mut self.observers {
            observer.on_processing_step(client_id, step, duration_us);
        }
    }
}

/// Performance monitoring observer
pub struct PerformanceMonitor {
    stats: Arc<Mutex<PerformanceStats>>,
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_frames_processed: usize,
    pub total_actions_sent: usize,
    pub average_frame_time_us: f32,
    pub frames_per_second: f32,
    pub last_fps_calculation: Instant,
    pub fps_frame_count: usize,

    // EWMA timing stats per step
    pub avg_scene_analysis_us: f32,
    pub avg_policy_inference_us: f32,
    pub avg_action_selection_us: f32,
    pub avg_macro_execution_us: f32,
    pub avg_reward_processing_us: f32,
    pub avg_experience_collection_us: f32,
    pub avg_image_change_detection_us: f32,
    pub avg_action_send_us: f32,

    // Max timing stats for bottleneck detection
    pub max_scene_analysis_us: u64,
    pub max_policy_inference_us: u64,
    pub max_action_selection_us: u64,
    pub max_macro_execution_us: u64,
    pub max_reward_processing_us: u64,
    pub max_experience_collection_us: u64,
    pub max_image_change_detection_us: u64,
    pub max_action_send_us: u64,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            total_frames_processed: 0,
            total_actions_sent: 0,
            average_frame_time_us: 0.0,
            frames_per_second: 0.0,
            last_fps_calculation: Instant::now(),
            fps_frame_count: 0,
            avg_scene_analysis_us: 0.0,
            avg_policy_inference_us: 0.0,
            avg_action_selection_us: 0.0,
            avg_macro_execution_us: 0.0,
            avg_reward_processing_us: 0.0,
            avg_experience_collection_us: 0.0,
            avg_image_change_detection_us: 0.0,
            avg_action_send_us: 0.0,
            max_scene_analysis_us: 0,
            max_policy_inference_us: 0,
            max_action_selection_us: 0,
            max_macro_execution_us: 0,
            max_reward_processing_us: 0,
            max_experience_collection_us: 0,
            max_image_change_detection_us: 0,
            max_action_send_us: 0,
        }
    }
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(PerformanceStats::default())),
        }
    }

    pub fn get_stats(&self) -> PerformanceStats {
        self.stats.lock().unwrap().clone()
    }

    pub fn get_stats_shared(&self) -> Arc<Mutex<PerformanceStats>> {
        Arc::clone(&self.stats)
    }

    fn update_ewma(current: f32, new_value: u64, alpha: f32) -> f32 {
        current * (1.0 - alpha) + new_value as f32 * alpha
    }
}

impl MetricsObserver for PerformanceMonitor {
    fn on_frame_processed(&mut self, _client_id: Uuid, metrics: &FrameMetrics) {
        let mut stats = self.stats.lock().unwrap();
        stats.total_frames_processed += 1;
        tracing::debug!(
            "PerformanceMonitor: processed frame {}, total_time={}us",
            stats.total_frames_processed,
            metrics.total_processing_duration_us
        );

        const ALPHA: f32 = 0.1; // EWMA smoothing factor
        stats.average_frame_time_us = Self::update_ewma(
            stats.average_frame_time_us,
            metrics.total_processing_duration_us,
            ALPHA,
        );

        // Update FPS calculation
        stats.fps_frame_count += 1;
        let now = Instant::now();
        let elapsed = now.duration_since(stats.last_fps_calculation);
        if elapsed.as_secs_f32() >= 1.0 {
            stats.frames_per_second = stats.fps_frame_count as f32 / elapsed.as_secs_f32();
            stats.fps_frame_count = 0;
            stats.last_fps_calculation = now;
        }
    }

    fn on_action_sent(&mut self, _client_id: Uuid, _action: GameAction) {
        let mut stats = self.stats.lock().unwrap();
        stats.total_actions_sent += 1;
    }

    fn on_processing_step(&mut self, _client_id: Uuid, step: ProcessingStepType, duration_us: u64) {
        let mut stats = self.stats.lock().unwrap();
        const ALPHA: f32 = 0.1;

        match step {
            ProcessingStepType::SceneAnalysis => {
                stats.avg_scene_analysis_us =
                    Self::update_ewma(stats.avg_scene_analysis_us, duration_us, ALPHA);
                stats.max_scene_analysis_us = stats.max_scene_analysis_us.max(duration_us);
            }
            ProcessingStepType::PolicyInference => {
                stats.avg_policy_inference_us =
                    Self::update_ewma(stats.avg_policy_inference_us, duration_us, ALPHA);
                stats.max_policy_inference_us = stats.max_policy_inference_us.max(duration_us);
            }
            ProcessingStepType::ActionSelection => {
                stats.avg_action_selection_us =
                    Self::update_ewma(stats.avg_action_selection_us, duration_us, ALPHA);
                stats.max_action_selection_us = stats.max_action_selection_us.max(duration_us);
            }
            ProcessingStepType::MacroExecution => {
                stats.avg_macro_execution_us =
                    Self::update_ewma(stats.avg_macro_execution_us, duration_us, ALPHA);
                stats.max_macro_execution_us = stats.max_macro_execution_us.max(duration_us);
            }
            ProcessingStepType::RewardProcessing => {
                stats.avg_reward_processing_us =
                    Self::update_ewma(stats.avg_reward_processing_us, duration_us, ALPHA);
                stats.max_reward_processing_us = stats.max_reward_processing_us.max(duration_us);
            }
            ProcessingStepType::ExperienceCollection => {
                stats.avg_experience_collection_us =
                    Self::update_ewma(stats.avg_experience_collection_us, duration_us, ALPHA);
                stats.max_experience_collection_us =
                    stats.max_experience_collection_us.max(duration_us);
            }
            ProcessingStepType::ImageChangeDetection => {
                stats.avg_image_change_detection_us =
                    Self::update_ewma(stats.avg_image_change_detection_us, duration_us, ALPHA);
                stats.max_image_change_detection_us =
                    stats.max_image_change_detection_us.max(duration_us);
            }
            ProcessingStepType::ActionSending => {
                stats.avg_action_send_us =
                    Self::update_ewma(stats.avg_action_send_us, duration_us, ALPHA);
                stats.max_action_send_us = stats.max_action_send_us.max(duration_us);
            }
        }
    }
}

/// Debug information tracker
pub struct DebugTracker {
    debug_info: Arc<Mutex<DebugInfo>>,
}

#[derive(Debug, Clone, Default)]
pub struct DebugInfo {
    pub last_client: Option<Uuid>,
    pub last_action_selection: Option<String>,
    pub current_macro: Option<String>,
    pub recent_frame_times: Vec<u64>,
    pub bottleneck_warnings: Vec<String>,
}

impl DebugTracker {
    pub fn new() -> Self {
        Self {
            debug_info: Arc::new(Mutex::new(DebugInfo::default())),
        }
    }

    pub fn get_debug_info(&self) -> DebugInfo {
        self.debug_info.lock().unwrap().clone()
    }

    pub fn get_debug_info_shared(&self) -> Arc<Mutex<DebugInfo>> {
        Arc::clone(&self.debug_info)
    }
}

impl MetricsObserver for DebugTracker {
    fn on_frame_processed(&mut self, client_id: Uuid, metrics: &FrameMetrics) {
        let mut debug = self.debug_info.lock().unwrap();
        debug.last_client = Some(client_id);

        // Keep recent frame times for debugging
        debug
            .recent_frame_times
            .push(metrics.total_processing_duration_us);
        if debug.recent_frame_times.len() > 10 {
            debug.recent_frame_times.remove(0);
        }

        // Detect bottlenecks (> 50ms processing time)
        if metrics.total_processing_duration_us > 50_000 {
            let warning = format!(
                "Slow frame processing: {}us for client {}",
                metrics.total_processing_duration_us, client_id
            );
            debug.bottleneck_warnings.push(warning);
            if debug.bottleneck_warnings.len() > 5 {
                debug.bottleneck_warnings.remove(0);
            }
        }
    }

    fn on_action_sent(&mut self, client_id: Uuid, action: GameAction) {
        let mut debug = self.debug_info.lock().unwrap();
        debug.last_action_selection = Some(format!("Client {}: {:?}", client_id, action));
    }

    fn on_processing_step(&mut self, _client_id: Uuid, step: ProcessingStepType, duration_us: u64) {
        // Detect step-level bottlenecks (> 20ms per step)
        if duration_us > 20_000 {
            let mut debug = self.debug_info.lock().unwrap();
            let warning = format!("Slow processing step {:?}: {}us", step, duration_us);
            debug.bottleneck_warnings.push(warning);
            if debug.bottleneck_warnings.len() > 5 {
                debug.bottleneck_warnings.remove(0);
            }
        }
    }
}
