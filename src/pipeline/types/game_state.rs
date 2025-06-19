use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum GameState {
    Unknown = 0,
    Intro = 1,
    MainMenu = 2,
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
