use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLPrediction {
    pub action_probabilities: Vec<f32>, // 12 buttons available.
    pub value_estimate: f32,
    pub confidence: f32,
}
