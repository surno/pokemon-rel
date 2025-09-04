/// Configuration options for the AI pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfiguration {
    pub action_selection_strategy: ActionSelectionStrategy,
    pub image_change_threshold: usize,
    pub image_change_history_window: usize,
    pub max_client_history: usize,
    pub policy_update_frequency: usize,
    pub performance_monitoring_enabled: bool,
    pub debug_tracking_enabled: bool,
}

impl Default for PipelineConfiguration {
    fn default() -> Self {
        Self {
            action_selection_strategy: ActionSelectionStrategy::Hybrid { policy_weight: 0.8 },
            image_change_threshold: 5,
            image_change_history_window: 5,
            max_client_history: 100,
            policy_update_frequency: 50,
            performance_monitoring_enabled: true,
            debug_tracking_enabled: true,
        }
    }
}

/// Different strategies for action selection
#[derive(Debug, Clone)]
pub enum ActionSelectionStrategy {
    /// Use only policy-based (PPO) action selection
    PolicyBased,
    /// Use only rule-based action selection
    RuleBased,
    /// Hybrid approach with configurable weight towards policy
    /// policy_weight: 0.0 = all rule-based, 1.0 = all policy-based
    Hybrid { policy_weight: f32 },
}

impl PipelineConfiguration {
    /// Create configuration optimized for performance
    pub fn performance_optimized() -> Self {
        Self {
            action_selection_strategy: ActionSelectionStrategy::PolicyBased,
            image_change_threshold: 8, // Less sensitive to reduce computation
            image_change_history_window: 3, // Smaller window for faster processing
            max_client_history: 50,    // Less history to save memory
            policy_update_frequency: 100, // Less frequent updates
            performance_monitoring_enabled: true,
            debug_tracking_enabled: false, // Disable debug for performance
        }
    }

    /// Create configuration optimized for learning
    pub fn learning_optimized() -> Self {
        Self {
            action_selection_strategy: ActionSelectionStrategy::Hybrid { policy_weight: 0.6 },
            image_change_threshold: 3, // More sensitive for better learning signals
            image_change_history_window: 7, // Larger window for stability
            max_client_history: 200,   // More history for learning
            policy_update_frequency: 25, // More frequent updates
            performance_monitoring_enabled: true,
            debug_tracking_enabled: true,
        }
    }

    /// Create configuration optimized for debugging
    pub fn debug_optimized() -> Self {
        Self {
            action_selection_strategy: ActionSelectionStrategy::RuleBased, // Deterministic for debugging
            image_change_threshold: 5,
            image_change_history_window: 5,
            max_client_history: 500,     // Lots of history for debugging
            policy_update_frequency: 10, // Frequent updates to see changes
            performance_monitoring_enabled: true,
            debug_tracking_enabled: true,
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.image_change_threshold == 0 {
            return Err("Image change threshold must be greater than 0".to_string());
        }

        if self.image_change_history_window == 0 {
            return Err("Image change history window must be greater than 0".to_string());
        }

        if self.max_client_history == 0 {
            return Err("Max client history must be greater than 0".to_string());
        }

        if self.policy_update_frequency == 0 {
            return Err("Policy update frequency must be greater than 0".to_string());
        }

        match &self.action_selection_strategy {
            ActionSelectionStrategy::Hybrid { policy_weight } => {
                if *policy_weight < 0.0 || *policy_weight > 1.0 {
                    return Err("Policy weight must be between 0.0 and 1.0".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }
}
