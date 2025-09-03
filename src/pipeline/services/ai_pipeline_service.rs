use crate::{
    error::AppError,
    pipeline::{EnrichedFrame, GameAction, services::learning::SmartActionService},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AIDecision {
    pub action: GameAction,
    pub confidence: f32,
    pub reasoning: String,
    pub timestamp: Instant,
    pub client_id: Uuid,
}

#[derive(Clone)]
pub struct AIPipelineService {
    smart_action_service: Arc<Mutex<SmartActionService>>,
    decision_history: HashMap<Uuid, Vec<AIDecision>>,
    action_tx: mpsc::Sender<(Uuid, GameAction)>,
    stats: AIStats,
}

#[derive(Debug, Clone)]
pub struct AIStats {
    pub total_frames_processed: usize,
    pub total_decisions_made: usize,
    pub average_confidence: f32,
    pub last_decision_time: Option<Instant>,
}

impl AIPipelineService {
    pub fn new(action_tx: mpsc::Sender<(Uuid, GameAction)>) -> Self {
        Self {
            smart_action_service: Arc::new(Mutex::new(SmartActionService::new())),
            decision_history: HashMap::new(),
            action_tx,
            stats: AIStats {
                total_frames_processed: 0,
                total_decisions_made: 0,
                average_confidence: 0.0,
                last_decision_time: None,
            },
        }
    }

    // Synchronous frame processing for use in GUI
    pub fn process_frame_sync(&mut self, frame: EnrichedFrame) -> Result<(), AppError> {
        // Create a simple runtime to process the frame
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| AppError::Client(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(self.process_frame(frame))
    }

    pub async fn process_frame(&mut self, frame: EnrichedFrame) -> Result<(), AppError> {
        let start_time = Instant::now();

        // Update stats
        self.stats.total_frames_processed += 1;
        self.stats.last_decision_time = Some(start_time);

        info!(
            "Processing frame for client {}: scene={:?}",
            frame.client,
            frame.state.as_ref().map(|s| s.scene)
        );

        // Get decision from smart action service
        let decision = {
            let mut smart_service = self.smart_action_service.lock().unwrap();
            let situation = smart_service.analyze_situation(&frame);
            smart_service.make_decision(&situation)
        };

        // Create AI decision record
        let ai_decision = AIDecision {
            action: decision.action.clone(),
            confidence: decision.confidence,
            reasoning: decision.reasoning.clone(),
            timestamp: start_time,
            client_id: frame.client,
        };

        // Store decision in history
        self.decision_history
            .entry(frame.client)
            .or_insert_with(Vec::new)
            .push(ai_decision.clone());

        // Keep history manageable (last 100 decisions per client)
        if let Some(history) = self.decision_history.get_mut(&frame.client) {
            if history.len() > 100 {
                history.remove(0);
            }
        }

        // Update stats
        self.stats.total_decisions_made += 1;
        self.update_average_confidence(decision.confidence);

        // Send action to the appropriate client
        let action_to_send = decision.action.clone();
        if let Err(e) = self.action_tx.send((frame.client, action_to_send)).await {
            warn!("Failed to send action to client {}: {}", frame.client, e);
        }

        info!(
            "AI Decision: {:?} (confidence: {:.2}) - {}",
            decision.action, decision.confidence, decision.reasoning
        );

        Ok(())
    }

    fn update_average_confidence(&mut self, new_confidence: f32) {
        let total = self.stats.total_decisions_made as f32;
        let current_avg = self.stats.average_confidence;
        self.stats.average_confidence = (current_avg * (total - 1.0) + new_confidence) / total;
    }

    pub fn get_stats(&self) -> AIStats {
        self.stats.clone()
    }

    pub fn get_client_decisions(&self, client_id: &Uuid) -> Vec<AIDecision> {
        self.decision_history
            .get(client_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_learning_stats(
        &self,
    ) -> Option<crate::pipeline::services::learning::smart_action_service::LearningStats> {
        let smart_service = self.smart_action_service.lock().ok()?;
        Some(smart_service.get_learning_stats())
    }

    pub async fn record_action_result(
        &self,
        client_id: Uuid,
        action: GameAction,
        was_successful: bool,
    ) -> Result<(), AppError> {
        // Get the last decision for this client to record the result
        if let Some(decisions) = self.decision_history.get(&client_id) {
            if let Some(last_decision) = decisions.last() {
                let mut smart_service = self.smart_action_service.lock().unwrap();

                // Create a mock situation for recording (in real implementation, you'd pass the actual situation)
                let mock_situation = crate::pipeline::services::learning::smart_action_service::GameSituation {
                    scene: crate::pipeline::types::Scene::Unknown,
                    has_text: false,
                    has_menu: false,
                    has_buttons: false,
                    dominant_colors: vec![],
                    urgency_level: crate::pipeline::services::learning::smart_action_service::UrgencyLevel::Low,
                };

                let action_clone = action.clone();
                smart_service.record_experience(mock_situation, action_clone, was_successful);
                info!(
                    "Recorded action result for client {}: {:?} -> success={}",
                    client_id, action, was_successful
                );
            }
        }
        Ok(())
    }
}

// Service implementation for processing frames
impl AIPipelineService {
    pub async fn start_frame_processing(
        mut self,
        mut frame_rx: mpsc::Receiver<EnrichedFrame>,
    ) -> Result<(), AppError> {
        info!("AI Pipeline Service started - waiting for frames...");

        while let Some(frame) = frame_rx.recv().await {
            if let Err(e) = self.process_frame(frame).await {
                error!("Error processing frame: {}", e);
            }
        }

        info!("AI Pipeline Service stopped");
        Ok(())
    }
}
