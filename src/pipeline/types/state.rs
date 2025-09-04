use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum Scene {
    Unknown = 0,
    Intro = 1,
    MainMenu = 2,
    Battle = 3,
    Overworld = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PokemonInfo {
    pub species: String,
    pub level: u32,
    pub hp_percentage: f32,
    pub is_shiny: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StoryProgress {
    GameStart,
    StarterObtained,
    FirstGym,
    SecondGym,
    ThirdGym,
    FourthGym,
    FifthGym,
    SixthGym,
    SeventhGym,
    EighthGym,
    EliteFour,
    Champion,
    PostGame,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocationType {
    Route,
    Town,
    City,
    Building,
    PokemonCenter,
    Gym,
    Cave,
    TallGrass,
    Water,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct State {
    pub scene: Scene,
    pub player_position: (f32, f32),
    pub pokemon_count: u32,

    // Pokemon Black specific state
    pub current_location: Option<String>, // "Route 1", "Accumula Town", etc.
    pub location_type: LocationType,      // What kind of area we're in
    pub pokemon_party: Vec<PokemonInfo>,  // Current party Pokemon
    pub pokedex_seen: u32,                // Pokemon seen count
    pub pokedex_caught: u32,              // Pokemon caught count
    pub badges_earned: u32,               // Gym badges (0-8)
    pub story_progress: StoryProgress,    // Story milestone
    pub in_tall_grass: bool,              // For encounter detection
    pub menu_cursor_position: Option<u32>, // Menu navigation state
    pub battle_turn: Option<u32>,         // Battle turn counter
    pub last_encounter_steps: u32,        // Steps since last wild Pokemon
    pub encounter_chain: u32,             // Chain for shiny hunting
}
