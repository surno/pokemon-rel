use crate::pipeline::services::learning::smart_action_service::{ActionDecision, GameSituation};
use crate::pipeline::{GameAction, MacroAction, RLPrediction};
use uuid::Uuid;

/// Strategy pattern for different action selection approaches
pub trait ActionSelector: Send + Sync {
    fn select_action(
        &mut self,
        client_id: Uuid,
        situation: &GameSituation,
        smart_decision: &ActionDecision,
        policy_prediction: Option<&RLPrediction>,
    ) -> ActionSelection;

    fn name(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct ActionSelection {
    pub game_action: GameAction,
    pub macro_action: MacroAction,
    pub confidence: f32,
    pub reasoning: String,
    pub selection_method: SelectionMethod,
}

#[derive(Debug, Clone)]
pub enum SelectionMethod {
    PolicyBased,
    RuleBased,
    Hybrid,
    Fallback,
}

/// Policy-based action selector using PPO predictions
pub struct PolicyBasedActionSelector;

impl ActionSelector for PolicyBasedActionSelector {
    fn select_action(
        &mut self,
        _client_id: Uuid,
        situation: &GameSituation,
        _smart_decision: &ActionDecision,
        policy_prediction: Option<&RLPrediction>,
    ) -> ActionSelection {
        if let Some(prediction) = policy_prediction {
            let policy_action = Self::sample_action_from_prediction(prediction);
            let macro_action = Self::map_action_to_macro(&policy_action, situation);

            ActionSelection {
                game_action: policy_action,
                macro_action,
                confidence: prediction.action_probabilities[policy_action as usize],
                reasoning: "Selected using PPO policy prediction".to_string(),
                selection_method: SelectionMethod::PolicyBased,
            }
        } else {
            // Fallback to random action
            let random_action = rand::random::<GameAction>();
            let macro_action = Self::map_action_to_macro(&random_action, situation);

            ActionSelection {
                game_action: random_action,
                macro_action,
                confidence: 0.1,
                reasoning: "No policy prediction available, using random action".to_string(),
                selection_method: SelectionMethod::Fallback,
            }
        }
    }

    fn name(&self) -> &'static str {
        "PolicyBasedActionSelector"
    }
}

impl PolicyBasedActionSelector {
    fn sample_action_from_prediction(pred: &RLPrediction) -> GameAction {
        use rand::distr::{Distribution, weighted::WeightedIndex};

        // Use first 11 actions (A, B, Up, Down, Left, Right, Start, Select, L, R, X)
        let mut probs: Vec<f32> = pred.action_probabilities.iter().copied().take(11).collect();
        if probs.is_empty() {
            return rand::random::<GameAction>();
        }
        if probs.iter().all(|&p| !p.is_finite() || p <= 0.0) {
            probs.fill(1.0);
        }
        let dist = match WeightedIndex::new(&probs) {
            Ok(d) => d,
            Err(_) => return rand::random::<GameAction>(),
        };
        let mut rng = rand::rng();
        let idx = dist.sample(&mut rng);
        Self::index_to_game_action(idx)
    }

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

    fn map_action_to_macro(action: &GameAction, situation: &GameSituation) -> MacroAction {
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
}

/// Rule-based action selector using smart action service decisions
pub struct RuleBasedActionSelector;

impl ActionSelector for RuleBasedActionSelector {
    fn select_action(
        &mut self,
        _client_id: Uuid,
        situation: &GameSituation,
        smart_decision: &ActionDecision,
        _policy_prediction: Option<&RLPrediction>,
    ) -> ActionSelection {
        let macro_action =
            PolicyBasedActionSelector::map_action_to_macro(&smart_decision.action, situation);

        ActionSelection {
            game_action: smart_decision.action.clone(),
            macro_action,
            confidence: smart_decision.confidence,
            reasoning: format!("Rule-based: {}", smart_decision.reasoning),
            selection_method: SelectionMethod::RuleBased,
        }
    }

    fn name(&self) -> &'static str {
        "RuleBasedActionSelector"
    }
}

/// Hybrid selector that combines policy and rule-based approaches
pub struct HybridActionSelector {
    policy_weight: f32,
}

impl HybridActionSelector {
    pub fn new(policy_weight: f32) -> Self {
        Self {
            policy_weight: policy_weight.clamp(0.0, 1.0),
        }
    }
}

impl ActionSelector for HybridActionSelector {
    fn select_action(
        &mut self,
        client_id: Uuid,
        situation: &GameSituation,
        smart_decision: &ActionDecision,
        policy_prediction: Option<&RLPrediction>,
    ) -> ActionSelection {
        // Use policy prediction with probability based on policy_weight
        if rand::random::<f32>() < self.policy_weight {
            let mut policy_selector = PolicyBasedActionSelector;
            let mut selection = policy_selector.select_action(
                client_id,
                situation,
                smart_decision,
                policy_prediction,
            );
            selection.selection_method = SelectionMethod::Hybrid;
            selection.reasoning = format!("Hybrid (policy): {}", selection.reasoning);
            selection
        } else {
            let mut rule_selector = RuleBasedActionSelector;
            let mut selection = rule_selector.select_action(
                client_id,
                situation,
                smart_decision,
                policy_prediction,
            );
            selection.selection_method = SelectionMethod::Hybrid;
            selection.reasoning = format!("Hybrid (rule): {}", selection.reasoning);
            selection
        }
    }

    fn name(&self) -> &'static str {
        "HybridActionSelector"
    }
}
