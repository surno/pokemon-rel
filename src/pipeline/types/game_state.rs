use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameState {
    Intro = 0,
    InGame = 1,
    Battle = 2,
    Menu = 3,
    Inventory = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateData {
    pub game_state: GameState,
    pub player_position: (f32, f32),
    pub pokemon_count: u32,
    // TODO: Add battle info
    // TODO: Add menu state
    // TODO: Add inventory state
}
