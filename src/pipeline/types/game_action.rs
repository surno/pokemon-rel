use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAction {
    pub action: String,
    pub value: f32,
}
