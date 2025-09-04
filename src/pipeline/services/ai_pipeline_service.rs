use crate::{
    error::AppError,
    pipeline::{
        EnrichedFrame, GameAction, MacroAction, RLService,
        services::learning::{
            experience_collector::ExperienceCollector,
            reward::{
                calculator::navigation_reward::NavigationRewardCalculator,
                processor::{
                    multi_objective_reward_processor::MultiObjectiveRewardProcessor,
                    reward_processor::RewardProcessor,
                },
            },
            smart_action_service::{GameSituation, SmartActionService},
        },
    },
};
use image::DynamicImage;
use imghash::{ImageHasher, perceptual::PerceptualHasher};
use rand::random;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::mpsc;
use tower::Service;
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
    decision_history: Arc<Mutex<HashMap<Uuid, Vec<AIDecision>>>>,
    action_tx: mpsc::Sender<(Uuid, GameAction)>,
    stats: AIStats,
    // Cache a downscaled copy of the last frame to avoid resizing every time
    last_action_and_situation: HashMap<Uuid, (GameAction, GameSituation, DynamicImage)>,
    image_hasher: Arc<PerceptualHasher>,
    hash_distance_history: HashMap<Uuid, VecDeque<usize>>, // rolling window per client
    // Simple tabular learner keyed by situation signature + macro action
    q_values: HashMap<(u64, MacroAction), f32>,
    epsilon: f32,
    last_success: HashMap<Uuid, bool>,
    active_macros: HashMap<Uuid, ActiveMacroState>,
    min_epsilon: f32,
    epsilon_decay: f32,
    failure_streak: HashMap<Uuid, u32>,
    stats_shared: Arc<Mutex<AIStats>>,
    debug_snapshot: Arc<Mutex<AIDebugSnapshot>>,
    // FPS tracking
    fps_window_start: Instant,
    fps_frames: usize,
    fps_decisions: usize,
    // Scene persistence tracking
    intro_scene_since: HashMap<Uuid, Instant>,
    // Learning components
    rl_service: RLService,
    reward_processor: Arc<Mutex<MultiObjectiveRewardProcessor>>,
    experience_collector: Arc<tokio::sync::Mutex<ExperienceCollector>>,
}

#[derive(Debug, Clone)]
pub struct AIStats {
    pub total_frames_processed: usize,
    pub total_decisions_made: usize,
    pub average_confidence: f32,
    pub last_decision_time: Option<Instant>,
    pub frames_per_sec: f32,
    pub decisions_per_sec: f32,
    pub total_actions_sent: usize,
}

#[derive(Debug, Clone, Default)]
pub struct AIDebugSnapshot {
    pub epsilon: f32,
    pub last_client: Option<Uuid>,
    pub active_macro: Option<(MacroAction, u32)>,
    pub failure_streak: u32,
    pub median_distance: Option<usize>,
}

impl AIPipelineService {
    fn candidate_macros() -> [MacroAction; 8] {
        [
            MacroAction::AdvanceDialog,
            MacroAction::WalkUp,
            MacroAction::WalkDown,
            MacroAction::WalkLeft,
            MacroAction::WalkRight,
            MacroAction::MenuSelect,
            MacroAction::MenuBack,
            MacroAction::PressStart,
        ]
    }
    fn situation_signature(situation: &GameSituation) -> u64 {
        // Very simple signature: pack booleans and scene into bits
        let mut sig: u64 = 0;
        let scene_val = match situation.scene {
            crate::pipeline::types::Scene::Unknown => 0u64,
            crate::pipeline::types::Scene::Intro => 1u64,
            crate::pipeline::types::Scene::MainMenu => 2u64,
            crate::pipeline::types::Scene::Battle => 3u64,
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
            GameAction::Start => MacroAction::PressStart,
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
            MacroAction::PressStart => GameAction::Start,
        }
    }

    fn select_macro_and_action(
        &mut self,
        client_id: Uuid,
        situation: &GameSituation,
        _default_action: &GameAction,
    ) -> (MacroAction, GameAction) {
        let sig = Self::situation_signature(situation);
        let candidates = Self::candidate_macros();

        // Special handling for Intro: strongly prefer PressStart; fallback after repeated failures
        let chosen_macro = if situation.scene == crate::pipeline::types::Scene::Intro {
            let streak = *self.failure_streak.get(&client_id).unwrap_or(&0);
            if streak < 10 {
                MacroAction::PressStart
            } else {
                MacroAction::AdvanceDialog
            }
        } else if random::<f32>() < self.epsilon {
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
            MacroAction::PressStart => 4,
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
        let (mac, act) = self.select_macro_and_action(client_id, situation, default_action);
        // Clamp walk duration if failing often
        let base_ticks = self.default_ticks_for_macro(mac);
        let ticks = if matches!(
            mac,
            MacroAction::WalkUp
                | MacroAction::WalkDown
                | MacroAction::WalkLeft
                | MacroAction::WalkRight
        ) {
            let s = *self.failure_streak.get(&client_id).unwrap_or(&0);
            if s >= 40 {
                2
            } else if s >= 20 {
                4
            } else {
                base_ticks
            }
        } else {
            base_ticks
        };
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

    fn update_q_values(&mut self, client_id: &Uuid, reward_override: Option<f32>) {
        // Requires at least one previous tuple stored in last_action_and_situation
        if let Some((last_action, last_situation, _)) =
            self.last_action_and_situation.get(client_id)
        {
            // Use provided reward if available; otherwise fall back to success heuristic
            let sig = Self::situation_signature(last_situation);
            let macro_act = self.map_action_to_macro(last_action, last_situation);
            let reward: f32 = if let Some(r) = reward_override {
                r
            } else {
                let success = *self.last_success.get(client_id).unwrap_or(&false);
                if success { 1.0 } else { -0.01 }
            };
            let alpha = 0.2f32;
            let q = self.q_values.entry((sig, macro_act)).or_insert(0.0);
            *q = *q + alpha * (reward - *q);
            // Update failure streak and epsilon decay
            let streak = self.failure_streak.entry(*client_id).or_insert(0);
            let success_now = reward > 0.0;
            if success_now {
                *streak = 0;
                self.epsilon = (self.epsilon * self.epsilon_decay).max(self.min_epsilon);
            } else {
                *streak = streak.saturating_add(1);
                if *streak % 20 == 0 {
                    self.epsilon = (self.epsilon + 0.02).min(0.5);
                }
            }
        }
    }
    pub fn new(action_tx: mpsc::Sender<(Uuid, GameAction)>) -> Self {
        let stats = AIStats {
            total_frames_processed: 0,
            total_decisions_made: 0,
            average_confidence: 0.0,
            last_decision_time: None,
            frames_per_sec: 0.0,
            decisions_per_sec: 0.0,
            total_actions_sent: 0,
        };
        let (training_tx, _training_rx) = mpsc::channel(1000);
        Self {
            smart_action_service: Arc::new(Mutex::new(SmartActionService::new())),
            decision_history: Arc::new(Mutex::new(HashMap::new())),
            action_tx,
            stats: stats.clone(),
            last_action_and_situation: HashMap::new(),
            image_hasher: Arc::new(PerceptualHasher::default()),
            hash_distance_history: HashMap::new(),
            q_values: HashMap::new(),
            epsilon: 0.2,
            last_success: HashMap::new(),
            active_macros: HashMap::new(),
            min_epsilon: 0.05,
            epsilon_decay: 0.999,
            failure_streak: HashMap::new(),
            stats_shared: Arc::new(Mutex::new(stats)),
            debug_snapshot: Arc::new(Mutex::new(AIDebugSnapshot::default())),
            fps_window_start: Instant::now(),
            fps_frames: 0,
            fps_decisions: 0,
            intro_scene_since: HashMap::new(),
            rl_service: RLService,
            reward_processor: Arc::new(Mutex::new(MultiObjectiveRewardProcessor::new(Box::new(
                NavigationRewardCalculator::default(),
            )))),
            experience_collector: Arc::new(tokio::sync::Mutex::new(ExperienceCollector::new(
                10_000,
                training_tx,
            ))),
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
        // Count a processed frame for FPS
        self.fps_frames += 1;

        // First, analyze the situation (brief lock)
        let current_situation = {
            let mut smart_service = self.smart_action_service.lock().unwrap();
            smart_service.analyze_situation(&frame)
        };

        // Compute image-change signal outside the lock; reuse cached downscaled last image
        if let Some((last_action, last_situation, last_small)) =
            self.last_action_and_situation.get(&client_id)
        {
            // Downscale current frame once (smaller for speed) and compare to last_small
            let small_curr = frame
                .image
                .resize(64, 64, image::imageops::FilterType::Nearest);
            let last_hash = self.image_hasher.hash_from_img(last_small);
            let current_hash = self.image_hasher.hash_from_img(&small_curr);
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

            // Success definition for Intro: if we move from Intro -> not Intro or menus/dialog appear
            let intro_skipped = last_situation.scene == crate::pipeline::types::Scene::Intro
                && current_situation.scene != crate::pipeline::types::Scene::Intro;
            let menu_or_dialog_now = current_situation.has_menu || current_situation.in_dialog;

            // Briefly lock to use SmartActionService's success heuristic and record experience
            let was_successful = {
                let smart_service = self.smart_action_service.lock().unwrap();
                smart_service.is_action_successful(last_situation, &current_situation)
            } && image_changed
                || intro_skipped
                || (last_situation.scene == crate::pipeline::types::Scene::Intro
                    && menu_or_dialog_now);

            self.last_success.insert(client_id, was_successful);
            {
                let mut smart_service = self.smart_action_service.lock().unwrap();
                smart_service.record_experience(
                    last_situation.clone(),
                    last_action.clone(),
                    was_successful,
                );
            }
            info!(
                "Client {}: Action {:?} was successful: {} (image changed: {})",
                client_id, last_action, was_successful, image_changed
            );
        }

        // Track Intro scene persistence window
        if current_situation.scene == crate::pipeline::types::Scene::Intro {
            self.intro_scene_since
                .entry(client_id)
                .or_insert(start_time);
        } else {
            self.intro_scene_since.remove(&client_id);
        }

        // Make a decision (brief lock)
        let decision = {
            let mut smart_service = self.smart_action_service.lock().unwrap();
            smart_service.make_decision(&current_situation)
        };

        let ai_decision = AIDecision {
            action: decision.action.clone(),
            confidence: decision.confidence,
            reasoning: decision.reasoning.clone(),
            timestamp: Instant::now(),
            client_id,
        };

        if let Ok(mut hist) = self.decision_history.lock() {
            hist.entry(client_id).or_default().push(ai_decision);
        }

        // Choose macro via epsilon-greedy and map to an action
        let selection_situation = current_situation.clone();
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
        // Generate RL prediction for current frame
        let prediction = self.rl_service.call(frame.clone()).await?;
        // Process reward and collect experience if available (avoid holding std::sync locks across await)
        let maybe_exp = {
            let mut rp = self.reward_processor.lock().unwrap();
            rp.process_frame(&frame, action_to_send.clone(), prediction.clone())
        };
        // Update Q-values using reward for the previous transition, before recording current as last
        let reward_for_prev = maybe_exp.as_ref().map(|e| e.reward);
        self.update_q_values(&client_id, reward_for_prev);
        if let Some(exp) = maybe_exp {
            let mut collector = self.experience_collector.lock().await;
            collector.collect_experience(exp).await;
        }
        // Now record current as the last action and situation for next step (cache downscaled image)
        let small_curr_for_cache = frame
            .image
            .resize(64, 64, image::imageops::FilterType::Nearest);
        self.last_action_and_situation.insert(
            client_id,
            (
                decision.action.clone(),
                selection_situation.clone(),
                small_curr_for_cache,
            ),
        );
        // If intro persists longer than 2s, force a PressStart action override
        if selection_situation.scene == crate::pipeline::types::Scene::Intro {
            if let Some(since) = self.intro_scene_since.get(&client_id) {
                if Instant::now().duration_since(*since).as_secs_f32() > 2.0 {
                    let forced = self.macro_to_action(MacroAction::PressStart);
                    info!(
                        "Intro persists >2s, forcing PressStart for client {}",
                        client_id
                    );
                    if let Err(e) = self.action_tx.try_send((client_id, forced)) {
                        warn!("Failed to send forced Start to client {}: {}", client_id, e);
                    }
                }
            }
        }
        if let Err(e) = self.action_tx.try_send((client_id, action_to_send)) {
            warn!("Failed to send action to client {}: {}", client_id, e);
        }
        self.stats.total_actions_sent += 1;

        info!(
            "AI Decision for client {}: {:?} (confidence: {:.2}) - {}",
            client_id, decision.action, decision.confidence, decision.reasoning
        );

        self.stats.total_decisions_made += 1;
        // Count a decision for FPS
        self.fps_decisions += 1;
        self.update_average_confidence(decision.confidence);
        self.stats.last_decision_time = Some(start_time);
        // Update FPS window
        let now = Instant::now();
        let elapsed = now.duration_since(self.fps_window_start);
        if elapsed.as_secs_f32() >= 1.0 {
            let secs = elapsed.as_secs_f32();
            self.stats.frames_per_sec = self.fps_frames as f32 / secs;
            self.stats.decisions_per_sec = self.fps_decisions as f32 / secs;
            self.fps_frames = 0;
            self.fps_decisions = 0;
            self.fps_window_start = now;
        }
        // mirror stats into shared copy for UI
        self.stats_shared
            .lock()
            .map(|mut s| *s = self.stats.clone())
            .ok();

        // Update debug snapshot for UI
        self.debug_snapshot
            .lock()
            .map(|mut snap| {
                snap.epsilon = self.epsilon;
                snap.last_client = Some(client_id);
                snap.active_macro = self
                    .active_macros
                    .get(&client_id)
                    .map(|st| (st.action, st.ticks_left));
                snap.failure_streak = *self.failure_streak.get(&client_id).unwrap_or(&0);
                snap.median_distance =
                    self.hash_distance_history.get(&client_id).and_then(|hist| {
                        if hist.is_empty() {
                            None
                        } else {
                            let mut v: Vec<usize> = hist.iter().copied().collect();
                            v.sort_unstable();
                            Some(v[v.len() / 2])
                        }
                    });
            })
            .ok();

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

    pub fn get_stats_shared(&self) -> AIStats {
        self.stats_shared
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| self.stats.clone())
    }

    pub fn get_debug_snapshot(&self) -> AIDebugSnapshot {
        self.debug_snapshot
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    pub fn get_client_decisions(&self, client_id: &Uuid) -> Vec<AIDecision> {
        self.decision_history
            .lock()
            .map(|m| m.get(client_id).cloned().unwrap_or_default())
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
        if let Ok(hist) = self.decision_history.lock() {
            if let Some(last_decision) = hist.get(&client_id).and_then(|v| v.last()) {
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
