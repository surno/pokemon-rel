use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub player_position: (f32, f32),
    pub pokemon_count: u32,
}
