use super::analyzers::{
    EnvironmentDetector, HPBarDetector, LocationDetector, MenuDetector, TextDetector,
};
/// Specialized scene detectors using Strategy pattern
use super::core::{
    DetectionContext, DetectionResult, DetectionSignalType, GameStateAnalyzer, SceneDetector,
    VisualDetector,
};
use crate::pipeline::types::{LocationType, StoryProgress};
use crate::pipeline::{Scene, State};
use std::time::Instant;

/// Battle scene detector - focuses on HP bars and battle UI
pub struct BattleSceneDetector {
    hp_bar_detector: HPBarDetector,
    menu_detector: MenuDetector,
}

impl BattleSceneDetector {
    pub fn new() -> Self {
        Self {
            hp_bar_detector: HPBarDetector::new(),
            menu_detector: MenuDetector::new(),
        }
    }
}

impl SceneDetector for BattleSceneDetector {
    fn detect_scene(&self, context: &DetectionContext) -> DetectionResult<Scene> {
        let start_time = Instant::now();

        // Check for HP bars (strongest battle indicator)
        let hp_result = self.hp_bar_detector.detect(context);
        let has_hp_bars = !hp_result.result.is_empty();

        // Check for battle menu
        let menu_result = self.menu_detector.detect(context);
        let has_battle_menu = !menu_result.result.is_empty();

        let confidence = if has_hp_bars && has_battle_menu {
            0.95 // Very confident if both present
        } else if has_hp_bars {
            0.85 // HP bars are strong indicator
        } else if has_battle_menu {
            0.6 // Menu alone is weaker
        } else {
            0.1 // No battle indicators
        };

        let is_battle = confidence > 0.5;
        let scene = if is_battle {
            Scene::Battle
        } else {
            Scene::Unknown
        };

        DetectionResult::new(
            scene,
            confidence,
            format!(
                "Battle detection: HP bars={}, menu={}",
                has_hp_bars, has_battle_menu
            ),
        )
        .with_timing(start_time)
    }

    fn name(&self) -> &'static str {
        "BattleSceneDetector"
    }

    fn supported_scenes(&self) -> Vec<Scene> {
        vec![Scene::Battle]
    }
}

/// Menu scene detector - focuses on main menu and text elements
pub struct MenuSceneDetector {
    text_detector: TextDetector,
}

impl MenuSceneDetector {
    pub fn new() -> Self {
        Self {
            text_detector: TextDetector::new().with_threshold(90),
        }
    }
}

impl SceneDetector for MenuSceneDetector {
    fn detect_scene(&self, context: &DetectionContext) -> DetectionResult<Scene> {
        let start_time = Instant::now();

        // Check for text blocks (menus typically have text)
        let text_result = self.text_detector.detect(context);
        let has_text = !text_result.result.is_empty();

        // Check if text is in menu-like arrangement
        let has_menu_layout = self.detect_menu_layout(context);

        let confidence = if has_text && has_menu_layout {
            0.8 // High confidence for menu
        } else if has_menu_layout {
            0.6 // Menu layout without text
        } else {
            0.2 // No menu indicators
        };

        let is_menu = confidence > 0.5;
        let scene = if is_menu {
            Scene::MainMenu
        } else {
            Scene::Unknown
        };

        DetectionResult::new(
            scene,
            confidence,
            format!(
                "Menu detection: text={}, layout={}",
                has_text, has_menu_layout
            ),
        )
        .with_timing(start_time)
    }

    fn name(&self) -> &'static str {
        "MenuSceneDetector"
    }

    fn supported_scenes(&self) -> Vec<Scene> {
        vec![Scene::MainMenu]
    }
}

impl MenuSceneDetector {
    fn detect_menu_layout(&self, context: &DetectionContext) -> bool {
        // Look for vertically arranged text blocks (menu options)
        let rgb = &context.rgb;
        let (width, height) = rgb.dimensions();
        let mut menu_lines = 0;

        let middle_start = height / 3;
        let middle_end = (height * 2) / 3;

        for y in (middle_start..middle_end).step_by(8) {
            let mut line_contrast = 0;
            for x in (0..width).step_by(8) {
                if self.pixel_has_menu_contrast(rgb, x, y) {
                    line_contrast += 1;
                }
            }

            if line_contrast > width / 32 {
                menu_lines += 1;
            }
        }

        menu_lines >= 3 // At least 3 menu lines
    }

    fn pixel_has_menu_contrast(&self, rgb: &image::RgbImage, x: u32, y: u32) -> bool {
        if let Some(pixel) = rgb.get_pixel_checked(x, y) {
            let [r, g, b] = pixel.0;
            let brightness = (r as u16 + g as u16 + b as u16) / 3;

            // Check for high contrast with neighbors
            for dy in 0..=1 {
                for dx in 0..=1 {
                    if let Some(neighbor) = rgb.get_pixel_checked(x + dx, y + dy) {
                        let [nr, ng, nb] = neighbor.0;
                        let neighbor_brightness = (nr as u16 + ng as u16 + nb as u16) / 3;
                        let contrast = brightness.abs_diff(neighbor_brightness);

                        if contrast > 80 {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

/// Overworld scene detector - focuses on location and environment
pub struct OverworldSceneDetector {
    location_detector: LocationDetector,
    environment_detector: EnvironmentDetector,
}

impl OverworldSceneDetector {
    pub fn new() -> Self {
        Self {
            location_detector: LocationDetector::new(),
            environment_detector: EnvironmentDetector::new(),
        }
    }
}

impl SceneDetector for OverworldSceneDetector {
    fn detect_scene(&self, context: &DetectionContext) -> DetectionResult<Scene> {
        let start_time = Instant::now();

        // Check for location signals
        let location_result = self.location_detector.detect(context);
        let has_location = !location_result.result.is_empty();

        // Check for environment signals
        let env_result = self.environment_detector.detect(context);
        let has_environment = !env_result.result.is_empty();

        // Overworld is detected by absence of strong UI elements and presence of environment
        let has_ui_elements = context.has_signal(DetectionSignalType::HPBar)
            || context.has_signal(DetectionSignalType::BattleMenu)
            || context.has_signal(DetectionSignalType::MainMenu);

        let confidence = if !has_ui_elements && (has_location || has_environment) {
            0.7 // Good confidence for overworld
        } else if !has_ui_elements {
            0.5 // Default to overworld if no UI elements
        } else {
            0.1 // UI elements present, probably not overworld
        };

        let is_overworld = confidence > 0.4;
        let scene = if is_overworld {
            Scene::Overworld
        } else {
            Scene::Unknown
        };

        DetectionResult::new(
            scene,
            confidence,
            format!(
                "Overworld detection: location={}, env={}, no_ui={}",
                has_location, has_environment, !has_ui_elements
            ),
        )
        .with_timing(start_time)
    }

    fn name(&self) -> &'static str {
        "OverworldSceneDetector"
    }

    fn supported_scenes(&self) -> Vec<Scene> {
        vec![Scene::Overworld]
    }
}

/// Intro scene detector - focuses on dialog and intro elements
pub struct IntroSceneDetector {
    text_detector: TextDetector,
}

impl IntroSceneDetector {
    pub fn new() -> Self {
        Self {
            text_detector: TextDetector::new().with_threshold(80),
        }
    }
}

impl SceneDetector for IntroSceneDetector {
    fn detect_scene(&self, context: &DetectionContext) -> DetectionResult<Scene> {
        let start_time = Instant::now();

        // Check for text (intro scenes are text-heavy)
        let text_result = self.text_detector.detect(context);
        let has_text = !text_result.result.is_empty();

        // Check for dialog box at bottom
        let has_dialog = self.detect_dialog_box(context);

        let confidence = if has_text && has_dialog {
            0.9 // Very confident for intro
        } else if has_text || has_dialog {
            0.6 // Moderate confidence
        } else {
            0.1 // No intro indicators
        };

        let is_intro = confidence > 0.5;
        let scene = if is_intro {
            Scene::Intro
        } else {
            Scene::Unknown
        };

        DetectionResult::new(
            scene,
            confidence,
            format!("Intro detection: text={}, dialog={}", has_text, has_dialog),
        )
        .with_timing(start_time)
    }

    fn name(&self) -> &'static str {
        "IntroSceneDetector"
    }

    fn supported_scenes(&self) -> Vec<Scene> {
        vec![Scene::Intro]
    }
}

impl IntroSceneDetector {
    fn detect_dialog_box(&self, context: &DetectionContext) -> bool {
        let rgb = &context.rgb;
        let (width, height) = rgb.dimensions();

        // Look for dialog box in bottom quarter
        let bottom_start = (height * 3) / 4;
        let mut border_pixels = 0;
        let mut total_pixels = 0;

        for y in bottom_start..height {
            for x in 0..width {
                if let Some(pixel) = rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    let brightness = (r as u16 + g as u16 + b as u16) / 3;

                    // Look for dialog box borders (dark lines)
                    if brightness < 50 {
                        border_pixels += 1;
                    }
                    total_pixels += 1;
                }
            }
        }

        total_pixels > 0 && border_pixels as f32 / total_pixels as f32 > 0.1
    }
}

/// Name creation scene detector - focuses on character input grids
pub struct NameCreationSceneDetector {
    text_detector: TextDetector,
    menu_detector: MenuDetector,
}

impl NameCreationSceneDetector {
    pub fn new() -> Self {
        Self {
            text_detector: TextDetector::new().with_threshold(70),
            menu_detector: MenuDetector::new(),
        }
    }
}

impl SceneDetector for NameCreationSceneDetector {
    fn detect_scene(&self, context: &DetectionContext) -> DetectionResult<Scene> {
        let start_time = Instant::now();

        // Check for text (name creation screens have prompts)
        let text_result = self.text_detector.detect(context);
        let has_text = !text_result.result.is_empty();

        // Check for menu/grid layout (character selection grid)
        let menu_result = self.menu_detector.detect(context);
        let has_menu = !menu_result.result.is_empty();

        // Check for character grid pattern (specific to name creation)
        let has_character_grid = self.detect_character_grid(context);

        // Check for name prompt text
        let has_name_prompt = self.detect_name_prompt(context);

        // Check for character count indicators (e.g., "_ _ _ _")
        let has_character_slots = self.detect_character_slots(context);

        let confidence = if has_character_grid && has_name_prompt {
            0.95 // Very confident - both grid and prompt detected
        } else if has_character_grid && (has_text || has_menu) {
            0.85 // High confidence - grid with supporting evidence
        } else if has_name_prompt && has_menu {
            0.75 // Good confidence - prompt with menu
        } else if has_character_slots && has_text {
            0.65 // Moderate confidence - slots with text
        } else if has_text && has_menu {
            0.4 // Low confidence - could be name creation or battle
        } else {
            0.1 // Very low confidence
        };

        let is_name_creation = confidence > 0.6;
        let scene = if is_name_creation {
            Scene::NameCreation
        } else {
            Scene::Unknown
        };

        DetectionResult::new(
            scene,
            confidence,
            format!(
                "Name creation detection: grid={}, prompt={}, slots={}, text={}, menu={}",
                has_character_grid, has_name_prompt, has_character_slots, has_text, has_menu
            ),
        )
        .with_timing(start_time)
    }

    fn name(&self) -> &'static str {
        "NameCreationSceneDetector"
    }

    fn supported_scenes(&self) -> Vec<Scene> {
        vec![Scene::NameCreation]
    }
}

impl NameCreationSceneDetector {
    /// Detect character grid pattern typical of Pokemon naming screens
    fn detect_character_grid(&self, context: &DetectionContext) -> bool {
        let (width, height) = context.dimensions;

        // Look for grid-like patterns in the center area
        let start_x = width / 4;
        let end_x = (width * 3) / 4;
        let start_y = height / 4;
        let end_y = (height * 3) / 4;

        let mut grid_transitions = 0;
        let mut total_samples = 0;

        // Sample in a grid pattern to detect character boundaries
        for y in (start_y..end_y).step_by(16) {
            let mut prev_brightness: Option<u8> = None;
            for x in (start_x..end_x).step_by(12) {
                if let Some(pixel) = context.rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    let brightness = ((r as u16 + g as u16 + b as u16) / 3) as u8;

                    if let Some(prev) = prev_brightness {
                        if (brightness as i16 - prev as i16).abs() > 40 {
                            grid_transitions += 1;
                        }
                    }
                    prev_brightness = Some(brightness);
                    total_samples += 1;
                }
            }
        }

        // If we have enough transitions, it suggests a grid layout
        total_samples > 0 && (grid_transitions as f32 / total_samples as f32) > 0.3
    }

    /// Detect text patterns that suggest name input prompts
    fn detect_name_prompt(&self, context: &DetectionContext) -> bool {
        // This is a simplified heuristic - in a real implementation,
        // you'd use OCR or pattern matching to look for specific text
        // like "Enter name", "What's your name?", etc.

        let (width, height) = context.dimensions;

        // Check top area where prompts typically appear
        let end_y = height / 3;
        let mut text_density = 0;
        let mut total_samples = 0;

        for y in (0..end_y).step_by(4) {
            for x in (0..width).step_by(4) {
                if let Some(pixel) = context.rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    let brightness = (r as u16 + g as u16 + b as u16) / 3;

                    // Look for text-like contrast patterns
                    if brightness < 60 || brightness > 220 {
                        text_density += 1;
                    }
                    total_samples += 1;
                }
            }
        }

        // High text density in top area suggests a prompt
        total_samples > 0 && (text_density as f32 / total_samples as f32) > 0.2
    }

    /// Detect character slot patterns (underscores or boxes for name length)
    fn detect_character_slots(&self, context: &DetectionContext) -> bool {
        let (width, height) = context.dimensions;

        // Look for horizontal line patterns that might be underscores
        let start_y = height / 3;
        let end_y = (height * 2) / 3;

        let mut horizontal_lines = 0;

        for y in (start_y..end_y).step_by(8) {
            let mut consecutive_dark = 0;
            let mut line_segments = 0;

            for x in (0..width).step_by(2) {
                if let Some(pixel) = context.rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    let brightness = (r as u16 + g as u16 + b as u16) / 3;

                    if brightness < 80 {
                        consecutive_dark += 1;
                    } else {
                        if consecutive_dark > 8 {
                            line_segments += 1;
                        }
                        consecutive_dark = 0;
                    }
                }
            }

            // Final check for line at end of row
            if consecutive_dark > 8 {
                line_segments += 1;
            }

            // Multiple line segments suggest character slots
            if line_segments >= 3 {
                horizontal_lines += 1;
            }
        }

        // If we found horizontal line patterns, likely character slots
        horizontal_lines > 0
    }
}

/// Pokemon-specific game state analyzer
pub struct PokemonStateAnalyzer {
    location_detector: LocationDetector,
    environment_detector: EnvironmentDetector,
}

impl PokemonStateAnalyzer {
    pub fn new() -> Self {
        Self {
            location_detector: LocationDetector::new(),
            environment_detector: EnvironmentDetector::new(),
        }
    }
}

impl GameStateAnalyzer for PokemonStateAnalyzer {
    fn analyze_state(&self, context: &DetectionContext, scene: Scene) -> DetectionResult<State> {
        let start_time = Instant::now();

        // Detect location type
        let location_type = self.detect_location_type(context, scene);

        // Detect environment features
        let in_tall_grass = context.has_signal(DetectionSignalType::TallGrass);

        // Create state with detected information
        let state = State {
            scene,
            player_position: (0.0, 0.0), // TODO: Implement position detection
            pokemon_count: 0,            // TODO: Implement party detection
            current_location: None,      // TODO: Implement location name detection
            location_type,
            pokemon_party: Vec::new(), // TODO: Implement party analysis
            pokedex_seen: 0,           // TODO: Implement pokedex detection
            pokedex_caught: 0,
            badges_earned: 0, // TODO: Implement badge detection
            story_progress: StoryProgress::GameStart,
            in_tall_grass,
            menu_cursor_position: None, // TODO: Implement cursor detection
            battle_turn: None,          // TODO: Implement battle turn detection
            last_encounter_steps: 0,
            encounter_chain: 0,
        };

        DetectionResult::new(state, 0.8, format!("State analysis for {:?} scene", scene))
            .with_timing(start_time)
    }

    fn name(&self) -> &'static str {
        "PokemonStateAnalyzer"
    }
}

impl PokemonStateAnalyzer {
    fn detect_location_type(&self, context: &DetectionContext, scene: Scene) -> LocationType {
        match scene {
            Scene::Battle => LocationType::Unknown,
            Scene::MainMenu => LocationType::Unknown,
            Scene::Intro => LocationType::Unknown,
            Scene::NameCreation => LocationType::Unknown,
            Scene::Overworld => {
                // Use location detector signals to determine location type
                if context.has_signal(DetectionSignalType::PokemonCenter) {
                    LocationType::PokemonCenter
                } else if context.has_signal(DetectionSignalType::Gym) {
                    LocationType::Gym
                } else if context.has_signal(DetectionSignalType::Cave) {
                    LocationType::Cave
                } else if context.has_signal(DetectionSignalType::City) {
                    LocationType::City
                } else if context.has_signal(DetectionSignalType::Town) {
                    LocationType::Town
                } else if context.has_signal(DetectionSignalType::Building) {
                    LocationType::Building
                } else if context.has_signal(DetectionSignalType::Water) {
                    LocationType::Water
                } else {
                    LocationType::Route // Default for overworld
                }
            }
            Scene::Unknown => LocationType::Unknown,
        }
    }
}

impl Default for BattleSceneDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MenuSceneDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for OverworldSceneDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for IntroSceneDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PokemonStateAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
