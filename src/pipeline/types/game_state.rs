use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub player_position: (f32, f32),
    pub pokemon_count: u32,
    // TODO: Add battle info
    // TODO: Add menu state
    // TODO: Add inventory state
}
