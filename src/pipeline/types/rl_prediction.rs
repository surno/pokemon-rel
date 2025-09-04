use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RLPrediction {
    pub action_probabilities: Vec<f32>,
    pub value_prediction: f32,
}
