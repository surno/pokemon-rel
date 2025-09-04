use crate::{
    error::AppError,
    pipeline::{
        EnrichedFrame, GameAction, MacroAction,
        services::learning::smart_action_service::{GameSituation, SmartActionService},
    },
};
use imghash::{ImageHasher, perceptual::PerceptualHasher};
use rand::random;
use std::{
    collections::{HashMap, VecDeque},
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

#[derive(Debug, Clone, Copy)]
struct ActiveMacroState {
    action: MacroAction,
    ticks_left: u32,
}

#[derive(Clone)]
pub struct AIPipelineService {
    smart_action_service: Arc<Mutex<SmartActionService>>,
    decision_history: HashMap<Uuid, Vec<AIDecision>>,
    action_tx: mpsc::Sender<(Uuid, GameAction)>,
    stats: AIStats,
    last_action_and_situation: HashMap<Uuid, (GameAction, GameSituation, EnrichedFrame)>,
    image_hasher: Arc<PerceptualHasher>,
    hash_distance_history: HashMap<Uuid, VecDeque<usize>>, // rolling window per client
    // Simple tabular learner keyed by situation signature + macro action
    q_values: HashMap<(u64, MacroAction), f32>,
    epsilon: f32,
    last_success: HashMap<Uuid, bool>,
    active_macros: HashMap<Uuid, ActiveMacroState>,
}

#[derive(Debug, Clone)]
pub struct AIStats {
    pub total_frames_processed: usize,
    pub total_decisions_made: usize,
    pub average_confidence: f32,
    pub last_decision_time: Option<Instant>,
}

impl AIPipelineService {
    fn candidate_macros() -> [MacroAction; 7] {
        [
            MacroAction::AdvanceDialog,
            MacroAction::WalkUp,
            MacroAction::WalkDown,
            MacroAction::WalkLeft,
            MacroAction::WalkRight,
            MacroAction::MenuSelect,
            MacroAction::MenuBack,
        ]
    }
    fn situation_signature(situation: &GameSituation) -> u64 {
        // Very simple signature: pack booleans and scene into bits
        let mut sig: u64 = 0;
        let scene_val = match situation.scene {
            crate::pipeline::types::Scene::Unknown => 0u64,
            crate::pipeline::types::Scene::Intro => 1u64,
            crate::pipeline::types::Scene::MainMenu => 2u64,
        };
        sig |= scene_val & 0xF;
        if situation.has_text {
            sig |= 1 << 4;
        }
        if situation.has_menu {
            sig |= 1 << 5;
        }
        if situation.has_buttons {
            sig |= 1 << 6;
        }
        if situation.in_dialog {
            sig |= 1 << 7;
        }
        if let Some(row) = situation.cursor_row {
            sig |= ((row as u64) & 0xFF) << 8;
        }
        sig
    }

    fn map_action_to_macro(&self, action: &GameAction, situation: &GameSituation) -> MacroAction {
        if situation.in_dialog || situation.has_text {
            return MacroAction::AdvanceDialog;
        }
        match action {
            GameAction::Up => MacroAction::WalkUp,
            GameAction::Down => MacroAction::WalkDown,
            GameAction::Left => MacroAction::WalkLeft,
            GameAction::Right => MacroAction::WalkRight,
            GameAction::B => MacroAction::MenuBack,
            _ => MacroAction::MenuSelect,
        }
    }

    fn macro_to_action(&self, mac: MacroAction) -> GameAction {
        match mac {
            MacroAction::AdvanceDialog => GameAction::A,
            MacroAction::WalkUp => GameAction::Up,
            MacroAction::WalkDown => GameAction::Down,
            MacroAction::WalkLeft => GameAction::Left,
            MacroAction::WalkRight => GameAction::Right,
            MacroAction::MenuSelect => GameAction::A,
            MacroAction::MenuBack => GameAction::B,
        }
    }

    fn select_macro_and_action(
        &mut self,
        situation: &GameSituation,
        _default_action: &GameAction,
    ) -> (MacroAction, GameAction) {
        let sig = Self::situation_signature(situation);
        let candidates = Self::candidate_macros();

        // Epsilon-greedy selection
        let chosen_macro = if random::<f32>() < self.epsilon {
            // explore
            let idx = (random::<u32>() as usize) % candidates.len();
            candidates[idx]
        } else {
            // exploit
            let mut best = candidates[0];
            let mut best_q = f32::MIN;
            for m in candidates.iter().copied() {
                let q = *self.q_values.get(&(sig, m)).unwrap_or(&0.0);
                if q > best_q {
                    best_q = q;
                    best = m;
                }
            }
            best
        };

        // Map macro to an immediate action
        let action = self.macro_to_action(chosen_macro);
        let final_action = action;
        (chosen_macro, final_action)
    }

    fn default_ticks_for_macro(&self, mac: MacroAction) -> u32 {
        match mac {
            MacroAction::AdvanceDialog => 1,
            MacroAction::MenuSelect => 1,
            MacroAction::MenuBack => 1,
            MacroAction::WalkUp
            | MacroAction::WalkDown
            | MacroAction::WalkLeft
            | MacroAction::WalkRight => 6,
        }
    }

    fn drive_macro_action(
        &mut self,
        client_id: Uuid,
        situation: &GameSituation,
        default_action: &GameAction,
        image_changed: bool,
    ) -> GameAction {
        // Peek current macro state immutably to decide early-stop without borrow conflicts
        if let Some(state_snapshot) = self.active_macros.get(&client_id).copied() {
            let mac = state_snapshot.action;
            let should_stop = match mac {
                MacroAction::AdvanceDialog | MacroAction::MenuSelect | MacroAction::MenuBack => {
                    // Stop if dialog/text/menu no longer visible
                    !(situation.in_dialog || situation.has_text || situation.has_menu)
                }
                _ => {
                    // Walking: stop if dialog/menu appears or significant change occurred
                    situation.in_dialog || situation.has_menu || image_changed
                }
            };
            if !should_stop && state_snapshot.ticks_left > 0 {
                // Now mutate: decrement ticks and continue
                if let Some(state) = self.active_macros.get_mut(&client_id) {
                    state.ticks_left -= 1;
                }
                let action = self.macro_to_action(mac);
                debug!(
                    "Continuing macro {:?}, ticks_left={}",
                    mac,
                    state_snapshot.ticks_left.saturating_sub(1)
                );
                return action;
            }
            // Remove finished or early-stopped macro
            let _ = self.active_macros.remove(&client_id);
        }

        // Select a new macro and initialize its ticks
        let (mac, act) = self.select_macro_and_action(situation, default_action);
        let ticks = self.default_ticks_for_macro(mac);
        let sig = Self::situation_signature(situation);
        let q = *self.q_values.get(&(sig, mac)).unwrap_or(&0.0);
        info!("Chose macro {:?} (ticks={}) with Q={:.3}", mac, ticks, q);
        self.active_macros.insert(
            client_id,
            ActiveMacroState {
                action: mac,
                ticks_left: ticks.saturating_sub(1),
            },
        );
        act
    }

    fn update_q_values(&mut self, client_id: &Uuid) {
        // Requires at least one previous tuple stored in last_action_and_situation
        if let Some((last_action, last_situation, _)) =
            self.last_action_and_situation.get(client_id)
        {
            // reward heuristic: use last image-change median and success flag we just computed in process_frame
            let sig = Self::situation_signature(last_situation);
            let macro_act = self.map_action_to_macro(last_action, last_situation);
            // Use last_success flag; otherwise small penalty
            let reward: f32 = if *self.last_success.get(client_id).unwrap_or(&false) {
                1.0
            } else {
                -0.01
            };
            let alpha = 0.2f32;
            let q = self.q_values.entry((sig, macro_act)).or_insert(0.0);
            *q = *q + alpha * (reward - *q);
        }
    }
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
            last_action_and_situation: HashMap::new(),
            image_hasher: Arc::new(PerceptualHasher::default()),
            hash_distance_history: HashMap::new(),
            q_values: HashMap::new(),
            epsilon: 0.2,
            last_success: HashMap::new(),
            active_macros: HashMap::new(),
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

        self.stats.total_frames_processed += 1;

        let (current_situation, decision) = {
            let mut smart_service = self.smart_action_service.lock().unwrap();
            let current_situation = smart_service.analyze_situation(&frame);

            if let Some((last_action, last_situation, last_frame)) =
                self.last_action_and_situation.get(&client_id)
            {
                let last_hash = self.image_hasher.hash_from_img(&last_frame.image);
                let current_hash = self.image_hasher.hash_from_img(&frame.image);
                let distance = last_hash.distance(&current_hash).unwrap_or(0);

                // Maintain rolling window of distances
                let history = self
                    .hash_distance_history
                    .entry(client_id)
                    .or_insert_with(|| VecDeque::with_capacity(5));
                if history.len() >= 5 {
                    let _ = history.pop_front();
                }
                history.push_back(distance);

                // Compute median distance for stability
                let mut sorted: Vec<usize> = history.iter().copied().collect();
                sorted.sort_unstable();
                let median_distance = sorted[sorted.len() / 2];
                let image_changed = median_distance > 5; // threshold can be tuned

                let was_successful = smart_service
                    .is_action_successful(last_situation, &current_situation)
                    && image_changed;
                self.last_success.insert(client_id, was_successful);
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

        self.last_action_and_situation.insert(
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

        self.decision_history
            .entry(client_id)
            .or_default()
            .push(ai_decision);

        // Update Q on last transition
        self.update_q_values(&client_id);

        // Choose macro via epsilon-greedy and map to an action
        let selection_situation = self
            .last_action_and_situation
            .get(&client_id)
            .map(|(_, s, _)| s.clone())
            .unwrap();
        // Use recent median distance to inform early stopping
        let image_changed_now = self
            .hash_distance_history
            .get(&client_id)
            .map(|hist| {
                if hist.is_empty() {
                    false
                } else {
                    let mut sorted: Vec<usize> = hist.iter().copied().collect();
                    sorted.sort_unstable();
                    sorted[sorted.len() / 2] > 5
                }
            })
            .unwrap_or(false);
        let action_to_send = self.drive_macro_action(
            client_id,
            &selection_situation,
            &decision.action,
            image_changed_now,
        );
        if let Err(e) = self.action_tx.send((client_id, action_to_send)).await {
            warn!("Failed to send action to client {}: {}", client_id, e);
        }

        info!(
            "AI Decision for client {}: {:?} (confidence: {:.2}) - {}",
            client_id, decision.action, decision.confidence, decision.reasoning
        );

        self.stats.total_decisions_made += 1;
        self.update_average_confidence(decision.confidence);
        self.stats.last_decision_time = Some(start_time);

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
                    in_dialog: false,
                    cursor_row: None,
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
