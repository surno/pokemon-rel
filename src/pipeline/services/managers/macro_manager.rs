use crate::pipeline::services::learning::smart_action_service::GameSituation;
use crate::pipeline::{GameAction, MacroAction};
use std::collections::HashMap;
use tracing::{debug, info};
use uuid::Uuid;

/// Manages macro action execution and state per client
/// Extracted from the monolithic AIPipelineService for better separation of concerns
pub struct MacroManager {
    active_macros: HashMap<Uuid, ActiveMacroState>,
}

#[derive(Debug, Clone, Copy)]
pub struct ActiveMacroState {
    pub action: MacroAction,
    pub ticks_left: u32,
}

impl MacroManager {
    pub fn new() -> Self {
        Self {
            active_macros: HashMap::new(),
        }
    }

    /// Execute macro logic for a client, handling continuation or starting new macros
    pub fn execute_macro(
        &mut self,
        client_id: Uuid,
        situation: &GameSituation,
        suggested_action: &GameAction,
        image_changed: bool,
    ) -> GameAction {
        // Check if we should continue current macro
        if let Some(state) = self.active_macros.get(&client_id).copied() {
            if self.should_continue_macro(state, situation, image_changed) && state.ticks_left > 0 {
                // Continue current macro
                self.decrement_macro_ticks(client_id);
                let action = self.macro_to_action(state.action);
                debug!(
                    "Continuing macro {:?} for client {}, ticks left: {}",
                    state.action,
                    client_id,
                    state.ticks_left.saturating_sub(1)
                );
                return action;
            } else {
                // Macro finished or should be stopped
                self.stop_macro(client_id);
            }
        }

        // Start new macro
        self.start_new_macro(client_id, situation, suggested_action)
    }

    /// Start a new macro for the client
    fn start_new_macro(
        &mut self,
        client_id: Uuid,
        situation: &GameSituation,
        suggested_action: &GameAction,
    ) -> GameAction {
        let macro_action = self.map_action_to_macro(suggested_action, situation);
        let ticks = self.default_ticks_for_macro(macro_action);

        info!(
            "Starting macro {:?} for client {} (ticks={})",
            macro_action, client_id, ticks
        );

        self.active_macros.insert(
            client_id,
            ActiveMacroState {
                action: macro_action,
                ticks_left: ticks.saturating_sub(1),
            },
        );

        self.macro_to_action(macro_action)
    }

    /// Check if current macro should continue
    fn should_continue_macro(
        &self,
        state: ActiveMacroState,
        situation: &GameSituation,
        image_changed: bool,
    ) -> bool {
        match state.action {
            MacroAction::AdvanceDialog | MacroAction::MenuSelect | MacroAction::MenuBack => {
                // Continue if dialog/text/menu is still visible
                situation.in_dialog || situation.has_text || situation.has_menu
            }
            _ => {
                // Walking macros: stop if dialog/menu appears or significant change occurred
                !(situation.in_dialog || situation.has_menu || image_changed)
            }
        }
    }

    /// Decrement ticks for active macro
    fn decrement_macro_ticks(&mut self, client_id: Uuid) {
        if let Some(state) = self.active_macros.get_mut(&client_id) {
            state.ticks_left = state.ticks_left.saturating_sub(1);
        }
    }

    /// Stop macro for client
    fn stop_macro(&mut self, client_id: Uuid) {
        if let Some(state) = self.active_macros.remove(&client_id) {
            debug!("Stopped macro {:?} for client {}", state.action, client_id);
        }
    }

    /// Map game action to appropriate macro action based on situation
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

    /// Convert macro action to game action
    fn macro_to_action(&self, macro_action: MacroAction) -> GameAction {
        match macro_action {
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

    /// Get default duration for macro actions
    fn default_ticks_for_macro(&self, macro_action: MacroAction) -> u32 {
        match macro_action {
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

    /// Get current macro state for a client
    pub fn get_macro_state(&self, client_id: &Uuid) -> Option<ActiveMacroState> {
        self.active_macros.get(client_id).copied()
    }

    /// Get all active macros (for debugging)
    pub fn get_all_active_macros(&self) -> &HashMap<Uuid, ActiveMacroState> {
        &self.active_macros
    }

    /// Force stop all macros for a client
    pub fn force_stop_client_macros(&mut self, client_id: Uuid) {
        self.stop_macro(client_id);
    }
}
