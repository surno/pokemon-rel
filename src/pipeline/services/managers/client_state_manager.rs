use crate::pipeline::services::learning::smart_action_service::{ActionDecision, GameSituation};
use crate::pipeline::{GameAction, Scene};
use image::DynamicImage;
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

/// Manages per-client state and history
/// Extracted from the monolithic AIPipelineService for better separation of concerns
pub struct ClientStateManager {
    client_states: HashMap<Uuid, ClientState>,
    decision_history: HashMap<Uuid, Vec<ActionDecision>>,
    max_history_per_client: usize,
}

#[derive(Debug, Clone)]
pub struct ClientState {
    pub last_action: Option<GameAction>,
    pub last_situation: Option<GameSituation>,
    pub last_small_image: Option<DynamicImage>,
    pub intro_scene_since: Option<Instant>,
    pub total_actions_taken: usize,
    pub last_update: Instant,
}

impl ClientState {
    pub fn new() -> Self {
        Self {
            last_action: None,
            last_situation: None,
            last_small_image: None,
            intro_scene_since: None,
            total_actions_taken: 0,
            last_update: Instant::now(),
        }
    }

    pub fn update_action_and_situation(
        &mut self,
        action: GameAction,
        situation: GameSituation,
        small_image: DynamicImage,
    ) {
        self.last_action = Some(action);
        self.last_situation = Some(situation);
        self.last_small_image = Some(small_image);
        self.total_actions_taken += 1;
        self.last_update = Instant::now();
    }

    pub fn update_intro_tracking(&mut self, current_scene: Scene) {
        if current_scene == Scene::Intro {
            // Start tracking intro scene if not already tracking
            if self.intro_scene_since.is_none() {
                self.intro_scene_since = Some(Instant::now());
            }
        } else {
            // Clear intro tracking when leaving intro scene
            self.intro_scene_since = None;
        }
    }

    pub fn get_intro_duration(&self) -> Option<std::time::Duration> {
        self.intro_scene_since
            .map(|since| Instant::now().duration_since(since))
    }

    pub fn is_intro_stuck(&self, threshold_seconds: f32) -> bool {
        self.get_intro_duration()
            .map(|duration| duration.as_secs_f32() > threshold_seconds)
            .unwrap_or(false)
    }
}

impl ClientStateManager {
    pub fn new() -> Self {
        Self {
            client_states: HashMap::new(),
            decision_history: HashMap::new(),
            max_history_per_client: 100, // Keep last 100 decisions per client
        }
    }

    pub fn with_max_history(mut self, max_history: usize) -> Self {
        self.max_history_per_client = max_history;
        self
    }

    /// Get or create client state
    pub fn get_or_create_client_state(&mut self, client_id: Uuid) -> &mut ClientState {
        self.client_states
            .entry(client_id)
            .or_insert_with(ClientState::new)
    }

    /// Get client state (read-only)
    pub fn get_client_state(&self, client_id: &Uuid) -> Option<&ClientState> {
        self.client_states.get(client_id)
    }

    /// Update client state with new action and situation
    pub fn update_client_state(
        &mut self,
        client_id: Uuid,
        action: GameAction,
        situation: GameSituation,
        small_image: DynamicImage,
    ) {
        let state = self.get_or_create_client_state(client_id);
        state.update_action_and_situation(action, situation, small_image);
    }

    /// Update intro scene tracking for a client
    pub fn update_intro_tracking(&mut self, client_id: Uuid, current_scene: Scene) {
        let state = self.get_or_create_client_state(client_id);
        state.update_intro_tracking(current_scene);
    }

    /// Check if client is stuck in intro scene
    pub fn is_client_intro_stuck(&self, client_id: &Uuid, threshold_seconds: f32) -> bool {
        self.get_client_state(client_id)
            .map(|state| state.is_intro_stuck(threshold_seconds))
            .unwrap_or(false)
    }

    /// Add decision to client history
    pub fn add_decision_to_history(&mut self, client_id: Uuid, decision: ActionDecision) {
        let history = self
            .decision_history
            .entry(client_id)
            .or_insert_with(Vec::new);
        history.push(decision);

        // Trim history if it gets too long
        if history.len() > self.max_history_per_client {
            history.remove(0);
        }
    }

    /// Get decision history for a client
    pub fn get_decision_history(&self, client_id: &Uuid) -> Vec<ActionDecision> {
        self.decision_history
            .get(client_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get recent decisions for a client
    pub fn get_recent_decisions(&self, client_id: &Uuid, count: usize) -> Vec<ActionDecision> {
        self.decision_history
            .get(client_id)
            .map(|history| history.iter().rev().take(count).cloned().collect())
            .unwrap_or_default()
    }

    /// Clear all data for a client (when client disconnects)
    pub fn clear_client_data(&mut self, client_id: &Uuid) {
        self.client_states.remove(client_id);
        self.decision_history.remove(client_id);
    }

    /// Get statistics about tracked clients
    pub fn get_stats(&self) -> ClientStateStats {
        let total_decisions: usize = self.decision_history.values().map(|h| h.len()).sum();
        let active_clients = self.client_states.len();

        ClientStateStats {
            active_clients,
            total_decisions_stored: total_decisions,
            max_history_per_client: self.max_history_per_client,
        }
    }

    /// Get all client IDs currently being tracked
    pub fn get_tracked_clients(&self) -> Vec<Uuid> {
        self.client_states.keys().copied().collect()
    }

    /// Check if any clients are stuck in intro
    pub fn get_intro_stuck_clients(&self, threshold_seconds: f32) -> Vec<Uuid> {
        self.client_states
            .iter()
            .filter_map(|(client_id, state)| {
                if state.is_intro_stuck(threshold_seconds) {
                    Some(*client_id)
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ClientStateStats {
    pub active_clients: usize,
    pub total_decisions_stored: usize,
    pub max_history_per_client: usize,
}

impl Default for ClientStateManager {
    fn default() -> Self {
        Self::new()
    }
}
