use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum Scene {
    Unknown = 0,
    Intro = 1,
    MainMenu = 2,
    Battle = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct State {
    pub scene: Scene,
    pub player_position: (f32, f32),
    pub pokemon_count: u32,
    // TODO: Add battle info
    // TODO: Add menu state
    // TODO: Add inventory state
}
