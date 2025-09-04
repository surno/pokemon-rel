use crate::{
    error::AppError,
    pipeline::{
        EnrichedFrame, GameAction, MacroAction, RLPrediction, RLService,
        services::{
            image::scene_annotation_service::SceneAnnotationService,
            learning::{
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
    },
};
use image::DynamicImage;
use imghash::{ImageHasher, perceptual::PerceptualHasher};
use rand::{distr::Distribution, distr::weighted::WeightedIndex, random};
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
    // Q-learning removed; policy-based selection only
    active_macros: HashMap<Uuid, ActiveMacroState>,
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
    // Scene annotation for proper scene detection
    scene_annotation_service: SceneAnnotationService,
    // When true, prefer policy actions (PPO) over tabular Q-learning selection
    use_policy: bool,
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
    // Timing metrics for bottleneck detection
    pub timing: TimingStats,
}

#[derive(Debug, Clone, Default)]
pub struct TimingStats {
    // EWMA (exponentially weighted moving average) timings in microseconds
    pub analyze_situation_us: f32,
    pub hash_distance_us: f32,
    pub policy_inference_us: f32,
    pub macro_selection_us: f32,
    pub reward_processing_us: f32,
    pub experience_collection_us: f32,
    pub action_send_us: f32,
    pub total_frame_us: f32,
    // Last frame timings for debugging spikes
    pub last_analyze_situation_us: u64,
    pub last_hash_distance_us: u64,
    pub last_policy_inference_us: u64,
    pub last_macro_selection_us: u64,
    pub last_reward_processing_us: u64,
    pub last_experience_collection_us: u64,
    pub last_action_send_us: u64,
    pub last_total_frame_us: u64,
    // Max timings to catch worst-case performance
    pub max_analyze_situation_us: u64,
    pub max_hash_distance_us: u64,
    pub max_policy_inference_us: u64,
    pub max_macro_selection_us: u64,
    pub max_reward_processing_us: u64,
    pub max_experience_collection_us: u64,
    pub max_action_send_us: u64,
    pub max_total_frame_us: u64,
}

#[derive(Debug, Clone, Default)]
pub struct AIDebugSnapshot {
    pub last_client: Option<Uuid>,
    pub active_macro: Option<(MacroAction, u32)>,
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
    // situation_signature removed with Q-learning

    fn index_to_game_action(idx: usize) -> GameAction {
        match idx {
            0 => GameAction::A,
            1 => GameAction::B,
            2 => GameAction::Up,
            3 => GameAction::Down,
            4 => GameAction::Left,
            5 => GameAction::Right,
            6 => GameAction::Start,
            7 => GameAction::Select,
            8 => GameAction::L,
            9 => GameAction::R,
            _ => GameAction::X,
        }
    }

    fn game_action_to_index(action: &GameAction) -> usize {
        match action {
            GameAction::A => 0,
            GameAction::B => 1,
            GameAction::Up => 2,
            GameAction::Down => 3,
            GameAction::Left => 4,
            GameAction::Right => 5,
            GameAction::Start => 6,
            GameAction::Select => 7,
            GameAction::L => 8,
            GameAction::R => 9,
            GameAction::X => 10,
        }
    }

    fn sample_action_from_prediction(pred: &RLPrediction) -> GameAction {
        // Use first 11 actions (A, B, Up, Down, Left, Right, Start, Select, L, R, X)
        let mut probs: Vec<f32> = pred.action_probabilities.iter().copied().take(11).collect();
        if probs.is_empty() {
            return random::<GameAction>();
        }
        if probs.iter().all(|&p| !p.is_finite() || p <= 0.0) {
            probs.fill(1.0);
        }
        let dist = match WeightedIndex::new(&probs) {
            Ok(d) => d,
            Err(_) => return random::<GameAction>(),
        };
        let mut rng = rand::rng();
        let idx = dist.sample(&mut rng);
        Self::index_to_game_action(idx)
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
        _client_id: Uuid,
        situation: &GameSituation,
        _default_action: &GameAction,
    ) -> (MacroAction, GameAction) {
        // Policy path only: map the suggested action into a macro directly
        let chosen_macro = self.map_action_to_macro(_default_action, situation);
        let action = self.macro_to_action(chosen_macro);
        (chosen_macro, action)
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
        let ticks = base_ticks;
        info!("Chose macro {:?} (ticks={})", mac, ticks);
        self.active_macros.insert(
            client_id,
            ActiveMacroState {
                action: mac,
                ticks_left: ticks.saturating_sub(1),
            },
        );
        act
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
            timing: TimingStats::default(),
        };
        let (training_tx, _training_rx) = mpsc::channel(1000);
        let this = Self {
            smart_action_service: Arc::new(Mutex::new(SmartActionService::new())),
            decision_history: Arc::new(Mutex::new(HashMap::new())),
            action_tx,
            stats: stats.clone(),
            last_action_and_situation: HashMap::new(),
            image_hasher: Arc::new(PerceptualHasher::default()),
            hash_distance_history: HashMap::new(),
            active_macros: HashMap::new(),
            stats_shared: Arc::new(Mutex::new(stats)),
            debug_snapshot: Arc::new(Mutex::new(AIDebugSnapshot::default())),
            fps_window_start: Instant::now(),
            fps_frames: 0,
            fps_decisions: 0,
            intro_scene_since: HashMap::new(),
            rl_service: RLService::new(),
            reward_processor: Arc::new(Mutex::new(MultiObjectiveRewardProcessor::new(Box::new(
                NavigationRewardCalculator::default(),
            )))),
            experience_collector: Arc::new(tokio::sync::Mutex::new(ExperienceCollector::new(
                10_000,
                training_tx,
            ))),
            scene_annotation_service: SceneAnnotationService::new(()),
            use_policy: true,
        };
        this
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
        let frame_start_time = Instant::now();
        let client_id = frame.client;

        self.stats.total_frames_processed += 1;
        info!(
            "Processing frame {} for client {}",
            self.stats.total_frames_processed, client_id
        );
        // Count a processed frame for FPS
        self.fps_frames += 1;

        // First, annotate the frame with scene detection
        let annotated_frame = match self.scene_annotation_service.call(frame).await {
            Ok(frame) => frame,
            Err(e) => {
                error!("Scene annotation failed: {}", e);
                return Err(e);
            }
        };

        // Then, analyze the situation (brief lock)
        let analyze_start = Instant::now();
        let current_situation = {
            let smart_service = self.smart_action_service.lock().unwrap();
            smart_service.analyze_situation(&annotated_frame)
        };
        let analyze_duration = analyze_start.elapsed().as_micros() as u64;
        Self::update_timing_stat(
            &mut self.stats.timing.analyze_situation_us,
            &mut self.stats.timing.last_analyze_situation_us,
            &mut self.stats.timing.max_analyze_situation_us,
            analyze_duration,
        );

        // Compute image-change signal outside the lock; reuse cached downscaled last image
        let hash_start = Instant::now();
        if let Some((last_action, last_situation, last_small)) =
            self.last_action_and_situation.get(&client_id)
        {
            // Downscale current frame once (smaller for speed) and compare to last_small
            let small_curr =
                annotated_frame
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
        let hash_duration = hash_start.elapsed().as_micros() as u64;
        Self::update_timing_stat(
            &mut self.stats.timing.hash_distance_us,
            &mut self.stats.timing.last_hash_distance_us,
            &mut self.stats.timing.max_hash_distance_us,
            hash_duration,
        );

        // Track Intro scene persistence window
        if current_situation.scene == crate::pipeline::types::Scene::Intro {
            self.intro_scene_since
                .entry(client_id)
                .or_insert(frame_start_time);
        } else {
            self.intro_scene_since.remove(&client_id);
        }

        // Make a decision (brief lock) for explainability/logging
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

        // PPO: get policy prediction for current frame and sample an action
        let policy_start = Instant::now();
        let prediction = self.rl_service.call(annotated_frame.clone()).await?;
        let policy_action = Self::sample_action_from_prediction(&prediction);
        let policy_duration = policy_start.elapsed().as_micros() as u64;
        Self::update_timing_stat(
            &mut self.stats.timing.policy_inference_us,
            &mut self.stats.timing.last_policy_inference_us,
            &mut self.stats.timing.max_policy_inference_us,
            policy_duration,
        );

        // Choose macro using either policy-guided mapping or tabular Q
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
        let macro_start = Instant::now();
        let action_to_send = self.drive_macro_action(
            client_id,
            &selection_situation,
            &policy_action,
            image_changed_now,
        );
        let macro_duration = macro_start.elapsed().as_micros() as u64;
        Self::update_timing_stat(
            &mut self.stats.timing.macro_selection_us,
            &mut self.stats.timing.last_macro_selection_us,
            &mut self.stats.timing.max_macro_selection_us,
            macro_duration,
        );
        // Process reward and collect experience if available (avoid holding std::sync locks across await)
        let reward_start = Instant::now();
        let maybe_exp = {
            let mut rp = self.reward_processor.lock().unwrap();
            rp.process_frame(&annotated_frame, action_to_send.clone(), prediction.clone())
        };
        let reward_duration = reward_start.elapsed().as_micros() as u64;
        Self::update_timing_stat(
            &mut self.stats.timing.reward_processing_us,
            &mut self.stats.timing.last_reward_processing_us,
            &mut self.stats.timing.max_reward_processing_us,
            reward_duration,
        );
        let exp_start = Instant::now();
        if let Some(exp) = maybe_exp.clone() {
            let mut collector = self.experience_collector.lock().await;
            collector.collect_experience(exp).await;
        }
        let exp_duration = exp_start.elapsed().as_micros() as u64;
        Self::update_timing_stat(
            &mut self.stats.timing.experience_collection_us,
            &mut self.stats.timing.last_experience_collection_us,
            &mut self.stats.timing.max_experience_collection_us,
            exp_duration,
        );
        // Online policy nudge (very small step) using reward as advantage proxy
        if let Some(exp) = maybe_exp {
            let action_idx = Self::game_action_to_index(&policy_action);
            self.rl_service.nudge_action(action_idx, exp.reward);
            // Periodically persist the policy
            if self.stats.total_actions_sent % 50 == 0 {
                self.rl_service.save_now_blocking();
            }
        }
        // Now record current as the last action and situation for next step (cache downscaled image)
        let small_curr_for_cache =
            annotated_frame
                .image
                .resize(64, 64, image::imageops::FilterType::Nearest);
        self.last_action_and_situation.insert(
            client_id,
            (
                action_to_send.clone(),
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
        let action_send_start = Instant::now();
        if let Err(e) = self.action_tx.try_send((client_id, action_to_send)) {
            warn!("Failed to send action to client {}: {}", client_id, e);
        }
        let action_send_duration = action_send_start.elapsed().as_micros() as u64;
        Self::update_timing_stat(
            &mut self.stats.timing.action_send_us,
            &mut self.stats.timing.last_action_send_us,
            &mut self.stats.timing.max_action_send_us,
            action_send_duration,
        );
        self.stats.total_actions_sent += 1;

        info!(
            "AI Decision for client {}: {:?} (confidence: {:.2}) - {}",
            client_id, decision.action, decision.confidence, decision.reasoning
        );

        self.stats.total_decisions_made += 1;
        // Count a decision for FPS
        self.fps_decisions += 1;
        self.update_average_confidence(decision.confidence);
        self.stats.last_decision_time = Some(frame_start_time);

        // Update total frame timing
        let total_frame_duration = frame_start_time.elapsed().as_micros() as u64;
        Self::update_timing_stat(
            &mut self.stats.timing.total_frame_us,
            &mut self.stats.timing.last_total_frame_us,
            &mut self.stats.timing.max_total_frame_us,
            total_frame_duration,
        );
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
                snap.last_client = Some(client_id);
                snap.active_macro = self
                    .active_macros
                    .get(&client_id)
                    .map(|st| (st.action, st.ticks_left));
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

    fn update_timing_stat(ewma: &mut f32, last: &mut u64, max: &mut u64, duration_us: u64) {
        const ALPHA: f32 = 0.1; // EWMA smoothing factor
        *ewma = *ewma * (1.0 - ALPHA) + duration_us as f32 * ALPHA;
        *last = duration_us;
        *max = (*max).max(duration_us);
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
}

// Q-learning persistence hooks removed

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
