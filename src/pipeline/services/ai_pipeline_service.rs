use crate::{
    error::AppError,
    pipeline::{
        EnrichedFrame, GameAction,
        services::learning::smart_action_service::{GameSituation, SmartActionService},
    },
};
use imghash::{ImageHasher, perceptual::PerceptualHasher};
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

struct AIPipelineState {
    decision_history: HashMap<Uuid, Vec<AIDecision>>,
    stats: AIStats,
    last_action_and_situation: HashMap<Uuid, (GameAction, GameSituation, EnrichedFrame)>,
}

#[derive(Clone)]
pub struct AIPipelineService {
    smart_action_service: Arc<Mutex<SmartActionService>>,
    state: Arc<Mutex<AIPipelineState>>,
    action_tx: mpsc::Sender<(Uuid, GameAction)>,
    image_hasher: Arc<PerceptualHasher>,
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
            state: Arc::new(Mutex::new(AIPipelineState {
                decision_history: HashMap::new(),
                stats: AIStats {
                    total_frames_processed: 0,
                    total_decisions_made: 0,
                    average_confidence: 0.0,
                    last_decision_time: None,
                },
                last_action_and_situation: HashMap::new(),
            })),
            action_tx,
            image_hasher: Arc::new(PerceptualHasher::default()),
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
        let client_id = frame.client;

        let mut state = self.state.lock().unwrap();
        state.stats.total_frames_processed += 1;

        let (current_situation, decision) = {
            let mut smart_service = self.smart_action_service.lock().unwrap();
            let current_situation = smart_service.analyze_situation(&frame);

            if let Some((last_action, last_situation, last_frame)) =
                state.last_action_and_situation.get(&client_id)
            {
                let last_hash = self.image_hasher.hash_from_img(&last_frame.image);
                let current_hash = self.image_hasher.hash_from_img(&frame.image);
                let image_changed = last_hash
                    .distance(&current_hash)
                    .map(|d| d > 5)
                    .unwrap_or(false); // treat errors as no change

                let was_successful = smart_service
                    .is_action_successful(last_situation, &current_situation)
                    && image_changed;
                smart_service.record_experience(
                    last_situation.clone(),
                    last_action.clone(),
                    was_successful,
                );
                info!(
                    "Client {}: Action {:?} was successful: {} (image changed: {})",
                    client_id, last_action, was_successful, image_changed
                );
            }

            let decision = smart_service.make_decision(&current_situation);
            (current_situation, decision)
        };

        state.last_action_and_situation.insert(
            client_id,
            (decision.action.clone(), current_situation, frame.clone()),
        );

        let ai_decision = AIDecision {
            action: decision.action.clone(),
            confidence: decision.confidence,
            reasoning: decision.reasoning.clone(),
            timestamp: start_time,
            client_id,
        };

        state
            .decision_history
            .entry(client_id)
            .or_default()
            .push(ai_decision);

        let action_to_send = decision.action.clone();
        if let Err(e) = self.action_tx.send((client_id, action_to_send)).await {
            warn!("Failed to send action to client {}: {}", client_id, e);
        }

        info!(
            "AI Decision for client {}: {:?} (confidence: {:.2}) - {}",
            client_id, decision.action, decision.confidence, decision.reasoning
        );

        state.stats.total_decisions_made += 1;
        let new_confidence = decision.confidence;
        let total = state.stats.total_decisions_made as f32;
        let current_avg = state.stats.average_confidence;
        state.stats.average_confidence = (current_avg * (total - 1.0) + new_confidence) / total;
        state.stats.last_decision_time = Some(start_time);

        Ok(())
    }

    pub fn get_stats(&self) -> AIStats {
        self.state.lock().unwrap().stats.clone()
    }

    pub fn get_client_decisions(&self, client_id: &Uuid) -> Vec<AIDecision> {
        self.state
            .lock()
            .unwrap()
            .decision_history
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
        if let Some(decisions) = self.state.lock().unwrap().decision_history.get(&client_id) {
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
