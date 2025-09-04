use crate::{
    error::AppError,
    pipeline::{
        EnrichedFrame, Scene, State,
        types::{LocationType, PokemonInfo, StoryProgress},
    },
};
use image::{DynamicImage, GrayImage, RgbImage};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;

#[derive(Debug, Clone)]
pub struct SceneAnnotationServiceBuilder {
    // Kept for API compatibility; no longer used
    _capacity: usize,
    _fp_rate: f64,
}

impl SceneAnnotationServiceBuilder {
    pub fn new(capacity: usize, fp_rate: f64) -> Self {
        Self {
            _capacity: capacity,
            _fp_rate: fp_rate,
        }
    }

    pub fn with_scene(mut self, scene: Scene, hashes: Vec<String>) -> Self {
        let _ = (scene, hashes); // no-op for compatibility
        self
    }

    pub fn build(self) -> SceneAnnotationService {
        SceneAnnotationService {}
    }
}

#[derive(Clone)]
pub struct SceneAnnotationService {}

impl SceneAnnotationService {
    pub fn new(_unused: ()) -> Self {
        Self {}
    }

    pub fn detect_scene_sync(&self, frame: &DynamicImage) -> crate::pipeline::Scene {
        self.detect_scene(frame)
    }

    fn detect_scene(&self, frame: &DynamicImage) -> Scene {
        // Simplified Pokemon scene detection with more logging
        let rgb = frame.to_rgb8();
        let (_width, _height) = rgb.dimensions();

        // Start with basic detection using the old simple methods
        let has_text = self.detect_text_simple(&rgb);
        let has_menu = self.detect_menu_simple(&rgb);
        let has_dialog = self.detect_dialog_box_bottom(&rgb);

        // Log what we detected for debugging
        tracing::debug!(
            "Detection results: has_text={}, has_menu={}, has_dialog={}",
            has_text,
            has_menu,
            has_dialog
        );

        // Use the original simple logic that was working
        if has_text && has_menu {
            tracing::debug!("Detected Battle scene (text + menu)");
            return Scene::Battle;
        }
        if has_menu {
            tracing::debug!("Detected MainMenu scene (menu only)");
            return Scene::MainMenu;
        }
        if has_text || has_dialog {
            tracing::debug!("Detected Intro scene (text or dialog)");
            return Scene::Intro;
        }

        // Default to overworld for Pokemon gameplay
        tracing::debug!("Defaulting to Overworld scene");
        Scene::Overworld
    }

    fn detect_battle_scene(&self, rgb: &RgbImage) -> bool {
        let (width, height) = rgb.dimensions();

        // Battle scenes typically have:
        // 1. HP bars in specific locations (top portion)
        // 2. Battle menu at bottom
        // 3. Specific color patterns

        // Check for HP bar indicators in top 25% of screen
        let hp_bar_found = self.detect_hp_bars(rgb, width, height);

        // Check for battle menu in bottom 25% of screen
        let battle_menu_found = self.detect_battle_menu(rgb, width, height);

        // Pokemon battle scenes often have these UI elements
        hp_bar_found || battle_menu_found
    }

    fn detect_hp_bars(&self, rgb: &RgbImage, width: u32, height: u32) -> bool {
        let top_quarter = height / 4;

        // Look for green/red horizontal bars (HP indicators)
        for y in 0..top_quarter {
            let mut consecutive_green = 0;
            let mut consecutive_red = 0;

            for x in 0..width {
                let pixel = rgb.get_pixel(x, y);
                let [r, g, b] = pixel.0;

                // Green HP bar detection (safe arithmetic)
                if g > 150 && g as u16 > r as u16 + 30 && g as u16 > b as u16 + 30 {
                    consecutive_green += 1;
                    consecutive_red = 0;
                }
                // Red HP bar detection (low HP, safe arithmetic)
                else if r > 150 && r as u16 > g as u16 + 30 && r as u16 > b as u16 + 30 {
                    consecutive_red += 1;
                    consecutive_green = 0;
                } else {
                    consecutive_green = 0;
                    consecutive_red = 0;
                }

                // If we found a long horizontal green or red line, likely HP bar
                if consecutive_green > width / 8 || consecutive_red > width / 8 {
                    return true;
                }
            }
        }
        false
    }

    fn detect_battle_menu(&self, rgb: &RgbImage, width: u32, height: u32) -> bool {
        let bottom_quarter_start = (height * 3) / 4;

        // Look for menu box patterns in bottom quarter
        let mut menu_boxes = 0;

        for y in (bottom_quarter_start..height).step_by(8) {
            for x in (0..width).step_by(16) {
                if self.detect_menu_box(rgb, x, y, 32, 16) {
                    menu_boxes += 1;
                }
            }
        }

        menu_boxes >= 2 // At least 2 menu boxes suggests battle interface
    }

    fn detect_main_menu(&self, rgb: &RgbImage) -> bool {
        let (width, height) = rgb.dimensions();

        // Main menu typically has:
        // 1. Title text/logo in upper portion
        // 2. Menu options in center/lower portion
        // 3. Specific background patterns

        // Check for large text blocks (title/logo)
        let has_title = self.detect_large_text_block(rgb, width, height / 3);

        // Check for menu options (smaller text blocks arranged vertically)
        let has_menu_options = self.detect_menu_options(rgb, width, height);

        has_title || has_menu_options
    }

    fn detect_large_text_block(&self, rgb: &RgbImage, width: u32, height_limit: u32) -> bool {
        let mut large_contrast_regions = 0;

        for y in (0..height_limit).step_by(4) {
            for x in (0..width).step_by(4) {
                if self.has_high_contrast_region(rgb, x, y, 24, 24) {
                    large_contrast_regions += 1;
                }
            }
        }

        // Title screens typically have many high-contrast regions
        large_contrast_regions > (width * height_limit) / 1000
    }

    fn detect_menu_options(&self, rgb: &RgbImage, width: u32, height: u32) -> bool {
        let middle_start = height / 3;
        let middle_end = (height * 2) / 3;

        let mut menu_lines = 0;

        for y in (middle_start..middle_end).step_by(8) {
            let mut line_contrast = 0;
            for x in (0..width).step_by(8) {
                if self.pixel_has_text_contrast(rgb, x, y) {
                    line_contrast += 1;
                }
            }

            // If this line has enough contrast, it might be a menu option
            if line_contrast > width / 32 {
                menu_lines += 1;
            }
        }

        menu_lines >= 3 // At least 3 menu lines suggests menu screen
    }

    fn detect_intro_dialog(&self, rgb: &RgbImage) -> bool {
        let (width, height) = rgb.dimensions();

        // Dialog scenes typically have:
        // 1. Text box at bottom of screen
        // 2. Character portraits or dialog indicators
        // 3. Specific UI patterns

        self.detect_dialog_box_bottom(rgb) || self.detect_text_heavy_scene(rgb)
    }

    fn detect_text_heavy_scene(&self, rgb: &RgbImage) -> bool {
        let (width, height) = rgb.dimensions();
        let mut text_regions = 0;

        // Sample the image for text-like patterns
        for y in (0..height).step_by(12) {
            for x in (0..width).step_by(12) {
                if self.pixel_has_text_contrast(rgb, x, y) {
                    text_regions += 1;
                }
            }
        }

        // If more than 15% of sampled regions look like text
        let total_samples = (width / 12) * (height / 12);
        text_regions > total_samples / 7
    }

    fn detect_menu_box(&self, rgb: &RgbImage, x: u32, y: u32, w: u32, h: u32) -> bool {
        let (img_w, img_h) = rgb.dimensions();
        if x + w >= img_w || y + h >= img_h {
            return false;
        }

        // Check if this region looks like a menu box (bordered rectangle)
        let mut border_pixels = 0;
        let mut total_border = 0;

        // Check top and bottom borders
        for dx in 0..w {
            total_border += 2;
            if self.pixel_looks_like_border(rgb, x + dx, y) {
                border_pixels += 1;
            }
            if y + h - 1 < img_h && self.pixel_looks_like_border(rgb, x + dx, y + h - 1) {
                border_pixels += 1;
            }
        }

        // Check left and right borders
        for dy in 1..h - 1 {
            total_border += 2;
            if self.pixel_looks_like_border(rgb, x, y + dy) {
                border_pixels += 1;
            }
            if x + w - 1 < img_w && self.pixel_looks_like_border(rgb, x + w - 1, y + dy) {
                border_pixels += 1;
            }
        }

        // If most border pixels look like borders, this is likely a menu box
        total_border > 0 && (border_pixels as f32 / total_border as f32) > 0.6
    }

    fn has_high_contrast_region(&self, rgb: &RgbImage, x: u32, y: u32, w: u32, h: u32) -> bool {
        let (img_w, img_h) = rgb.dimensions();
        if x + w >= img_w || y + h >= img_h {
            return false;
        }

        let mut contrast_pixels = 0;
        let mut total_pixels = 0;

        for dy in 0..h {
            for dx in 0..w {
                if x + dx + 1 < img_w && y + dy + 1 < img_h {
                    total_pixels += 1;
                    if self.pixel_has_text_contrast(rgb, x + dx, y + dy) {
                        contrast_pixels += 1;
                    }
                }
            }
        }

        total_pixels > 0 && (contrast_pixels as f32 / total_pixels as f32) > 0.3
    }

    fn pixel_has_text_contrast(&self, rgb: &RgbImage, x: u32, y: u32) -> bool {
        let (width, height) = rgb.dimensions();
        if x == 0 || y == 0 || x >= width - 1 || y >= height - 1 {
            return false;
        }

        let current = rgb.get_pixel(x, y);
        let right = rgb.get_pixel(x + 1, y);
        let down = rgb.get_pixel(x, y + 1);

        let curr_brightness = (current[0] as f32 + current[1] as f32 + current[2] as f32) / 3.0;
        let right_brightness = (right[0] as f32 + right[1] as f32 + right[2] as f32) / 3.0;
        let down_brightness = (down[0] as f32 + down[1] as f32 + down[2] as f32) / 3.0;

        (curr_brightness - right_brightness).abs() > 40.0
            || (curr_brightness - down_brightness).abs() > 40.0
    }

    fn pixel_looks_like_border(&self, rgb: &RgbImage, x: u32, y: u32) -> bool {
        let pixel = rgb.get_pixel(x, y);
        let [r, g, b] = pixel.0;

        // Border pixels are typically dark or very light
        let brightness = (r as f32 + g as f32 + b as f32) / 3.0;
        brightness < 50.0 || brightness > 200.0
    }

    /// Analyze Pokemon Black specific game state from the current frame
    fn analyze_pokemon_black_state(&self, image: &DynamicImage, scene: Scene) -> State {
        let rgb = image.to_rgb8();

        // Detect location type based on visual cues
        let location_type = self.detect_location_type(&rgb, scene);

        // Detect if player is in tall grass (important for encounters)
        let in_tall_grass = self.detect_tall_grass(&rgb);

        // Try to read location name from screen (if visible)
        let current_location = self.detect_location_name(&rgb);

        // Detect menu cursor position for menu navigation
        let menu_cursor_position = if scene == Scene::MainMenu {
            self.detect_menu_cursor(&rgb)
        } else {
            None
        };

        // Detect battle turn counter
        let battle_turn = if scene == Scene::Battle {
            self.detect_battle_turn(&rgb)
        } else {
            None
        };

        State {
            scene,
            player_position: (0.0, 0.0), // TODO: Implement position detection
            pokemon_count: 0,            // TODO: Implement party detection
            current_location,
            location_type,
            pokemon_party: Vec::new(), // TODO: Implement party analysis
            pokedex_seen: 0,           // TODO: Implement pokedex detection
            pokedex_caught: 0,
            badges_earned: 0, // TODO: Implement badge detection
            story_progress: StoryProgress::GameStart,
            in_tall_grass,
            menu_cursor_position,
            battle_turn,
            last_encounter_steps: 0,
            encounter_chain: 0,
        }
    }

    /// Detect what type of location we're in based on visual cues
    fn detect_location_type(&self, rgb: &RgbImage, scene: Scene) -> LocationType {
        match scene {
            Scene::Battle => LocationType::Unknown, // Could be anywhere
            Scene::MainMenu => LocationType::Unknown,
            Scene::Intro => LocationType::Unknown,
            Scene::Overworld => {
                // Analyze overworld visuals to determine location type
                if self.detect_pokemon_center_interior(rgb) {
                    LocationType::PokemonCenter
                } else if self.detect_gym_interior(rgb) {
                    LocationType::Gym
                } else if self.detect_building_interior(rgb) {
                    LocationType::Building
                } else if self.detect_water_area(rgb) {
                    LocationType::Water
                } else if self.detect_cave_area(rgb) {
                    LocationType::Cave
                } else if self.detect_city_area(rgb) {
                    LocationType::City
                } else if self.detect_town_area(rgb) {
                    LocationType::Town
                } else {
                    LocationType::Route // Default for overworld
                }
            }
            Scene::Unknown => LocationType::Unknown,
        }
    }

    /// Detect if player is currently in tall grass (key for Pokemon encounters)
    fn detect_tall_grass(&self, rgb: &RgbImage) -> bool {
        let (width, height) = rgb.dimensions();

        // Look for green grass patterns around the center of screen (where player is)
        let center_x = width / 2;
        let center_y = height / 2;
        let search_radius = 20;

        let mut grass_pixels = 0;
        let mut total_pixels = 0;

        for dy in 0..search_radius {
            for dx in 0..search_radius {
                let x = center_x.saturating_sub(search_radius / 2) + dx;
                let y = center_y.saturating_sub(search_radius / 2) + dy;

                if x < width && y < height {
                    total_pixels += 1;
                    let pixel = rgb.get_pixel(x, y);
                    let [r, g, b] = pixel.0;

                    // Detect grass-like green colors
                    if g > 100 && g as u16 > r as u16 + 20 && g as u16 > b as u16 + 10 {
                        grass_pixels += 1;
                    }
                }
            }
        }

        // If more than 30% of center area is grass-colored, likely in tall grass
        total_pixels > 0 && (grass_pixels as f32 / total_pixels as f32) > 0.3
    }

    /// Try to detect location name from screen text (Pokemon games show location names)
    fn detect_location_name(&self, _rgb: &RgbImage) -> Option<String> {
        // TODO: Implement OCR or pattern matching for location names
        // For now, return None - this would require more sophisticated text detection
        None
    }

    /// Detect menu cursor position for better menu navigation
    fn detect_menu_cursor(&self, rgb: &RgbImage) -> Option<u32> {
        let (_width, height) = rgb.dimensions();

        // Look for cursor indicators (arrows, highlights) in menu areas
        for y in (height / 3)..(2 * height / 3) {
            for x in 10..50 {
                // Left side where cursors usually appear
                let pixel = rgb.get_pixel(x, y);
                let [r, g, b] = pixel.0;

                // Look for bright cursor colors (white, yellow, blue)
                if (r > 200 && g > 200 && b > 200) || // White
                   (r > 200 && g > 200 && b < 100) || // Yellow
                   (r < 100 && g < 100 && b > 200)
                {
                    // Blue
                    // Rough cursor position based on Y coordinate
                    return Some((y - height / 3) / 20); // Approximate menu item
                }
            }
        }
        None
    }

    /// Detect battle turn counter
    fn detect_battle_turn(&self, _rgb: &RgbImage) -> Option<u32> {
        // TODO: Implement battle turn detection from UI elements
        None
    }

    // Location type detection helpers
    fn detect_pokemon_center_interior(&self, rgb: &RgbImage) -> bool {
        // Pokemon Centers have distinctive pink/red color schemes
        let (width, height) = rgb.dimensions();
        let mut pink_pixels = 0;
        let mut total_sampled = 0;

        for y in (0..height).step_by(8) {
            for x in (0..width).step_by(8) {
                total_sampled += 1;
                let pixel = rgb.get_pixel(x, y);
                let [r, g, b] = pixel.0;

                // Pink/red color detection for Pokemon Center (safe arithmetic)
                if r > 150 && r as u16 > g as u16 + 30 && r as u16 > b as u16 + 20 {
                    pink_pixels += 1;
                }
            }
        }

        total_sampled > 0 && (pink_pixels as f32 / total_sampled as f32) > 0.15
    }

    fn detect_gym_interior(&self, rgb: &RgbImage) -> bool {
        // Gyms often have distinctive architectural patterns and colors
        // Look for geometric patterns and specific color schemes
        let has_geometric_patterns = self.detect_geometric_patterns(rgb);
        let has_gym_colors = self.detect_gym_color_scheme(rgb);

        has_geometric_patterns && has_gym_colors
    }

    fn detect_building_interior(&self, rgb: &RgbImage) -> bool {
        // Indoor areas typically have:
        // - Walls and floors with specific patterns
        // - Different lighting than outdoor areas
        // - Furniture and indoor objects

        let has_indoor_lighting = self.detect_indoor_lighting(rgb);
        let has_walls = self.detect_wall_patterns(rgb);

        has_indoor_lighting || has_walls
    }

    fn detect_water_area(&self, rgb: &RgbImage) -> bool {
        let (width, height) = rgb.dimensions();
        let mut blue_pixels = 0;
        let mut total_sampled = 0;

        // Sample the image for blue water colors
        for y in (0..height).step_by(6) {
            for x in (0..width).step_by(6) {
                total_sampled += 1;
                let pixel = rgb.get_pixel(x, y);
                let [r, g, b] = pixel.0;

                // Water is typically blue
                if b > 120 && b as u16 > r as u16 + 20 && b as u16 > g as u16 + 10 {
                    blue_pixels += 1;
                }
            }
        }

        total_sampled > 0 && (blue_pixels as f32 / total_sampled as f32) > 0.4
    }

    fn detect_cave_area(&self, rgb: &RgbImage) -> bool {
        // Caves are typically dark with brown/gray colors
        let (width, height) = rgb.dimensions();
        let mut dark_pixels = 0;
        let mut total_sampled = 0;

        for y in (0..height).step_by(8) {
            for x in (0..width).step_by(8) {
                total_sampled += 1;
                let pixel = rgb.get_pixel(x, y);
                let [r, g, b] = pixel.0;

                let brightness = (r as u16 + g as u16 + b as u16) / 3;

                // Dark colors typical of caves
                if brightness < 80 {
                    dark_pixels += 1;
                }
            }
        }

        total_sampled > 0 && (dark_pixels as f32 / total_sampled as f32) > 0.6
    }

    fn detect_city_area(&self, rgb: &RgbImage) -> bool {
        // Cities have buildings, roads, more complex layouts
        let has_buildings = self.detect_building_structures(rgb);
        let has_roads = self.detect_road_patterns(rgb);

        has_buildings || has_roads
    }

    fn detect_town_area(&self, rgb: &RgbImage) -> bool {
        // Towns are smaller than cities, have houses but less complex
        let has_houses = self.detect_house_patterns(rgb);
        let has_simple_layout = !self.detect_complex_layout(rgb);

        has_houses && has_simple_layout
    }

    // Helper methods for location detection
    fn detect_geometric_patterns(&self, rgb: &RgbImage) -> bool {
        // Look for repeating geometric patterns typical of gyms
        let (width, height) = rgb.dimensions();
        let mut pattern_score = 0;

        // Sample for regular patterns
        for y in (0..height).step_by(16) {
            for x in (0..width).step_by(16) {
                if self.has_regular_pattern(rgb, x, y, 16) {
                    pattern_score += 1;
                }
            }
        }

        pattern_score > 3 // Threshold for geometric patterns
    }

    fn detect_gym_color_scheme(&self, rgb: &RgbImage) -> bool {
        // Gyms often have specific color themes (varies by gym)
        // For now, detect any strong single-color dominance
        let color_analysis = self.analyze_dominant_colors(rgb);
        color_analysis.has_strong_theme
    }

    fn detect_indoor_lighting(&self, rgb: &RgbImage) -> bool {
        let (width, height) = rgb.dimensions();
        let mut bright_pixels = 0;
        let mut total_sampled = 0;

        // Indoor areas often have artificial lighting (brighter, more uniform)
        for y in (0..height).step_by(12) {
            for x in (0..width).step_by(12) {
                total_sampled += 1;
                let pixel = rgb.get_pixel(x, y);
                let [r, g, b] = pixel.0;

                let brightness = (r as u16 + g as u16 + b as u16) / 3;

                // Indoor lighting tends to be in mid-range brightness
                if brightness > 100 && brightness < 200 {
                    bright_pixels += 1;
                }
            }
        }

        total_sampled > 0 && (bright_pixels as f32 / total_sampled as f32) > 0.5
    }

    fn detect_wall_patterns(&self, rgb: &RgbImage) -> bool {
        // Look for straight lines and rectangular patterns typical of indoor walls
        let (width, height) = rgb.dimensions();

        // Check for horizontal and vertical line patterns
        let horizontal_lines = self.count_horizontal_lines(rgb, width, height);
        let vertical_lines = self.count_vertical_lines(rgb, width, height);

        horizontal_lines > 2 || vertical_lines > 2
    }

    fn detect_building_structures(&self, rgb: &RgbImage) -> bool {
        // Cities have rectangular building shapes
        self.detect_rectangular_structures(rgb)
    }

    fn detect_road_patterns(&self, rgb: &RgbImage) -> bool {
        // Roads are typically gray/brown straight lines
        let (width, height) = rgb.dimensions();
        let mut road_pixels = 0;
        let mut total_sampled = 0;

        for y in (0..height).step_by(4) {
            for x in (0..width).step_by(4) {
                total_sampled += 1;
                let pixel = rgb.get_pixel(x, y);
                let [r, g, b] = pixel.0;

                // Road colors (gray, brown)
                let is_gray = (r as i16 - g as i16).abs() < 20 && (g as i16 - b as i16).abs() < 20;
                let is_brown = r > 100 && g > 80 && b < 80;

                if is_gray || is_brown {
                    road_pixels += 1;
                }
            }
        }

        total_sampled > 0 && (road_pixels as f32 / total_sampled as f32) > 0.2
    }

    fn detect_house_patterns(&self, rgb: &RgbImage) -> bool {
        // Houses have distinctive shapes and colors
        self.detect_rectangular_structures(rgb) && !self.detect_large_structures(rgb)
    }

    fn detect_complex_layout(&self, rgb: &RgbImage) -> bool {
        // Complex layouts have more variety in structures and patterns
        let structure_variety = self.count_structure_variety(rgb);
        structure_variety > 5
    }

    // Utility methods for pattern detection
    fn has_regular_pattern(&self, rgb: &RgbImage, x: u32, y: u32, size: u32) -> bool {
        let (width, height) = rgb.dimensions();
        if x + size >= width || y + size >= height {
            return false;
        }

        // Check if this region has repeating patterns
        let mut pattern_consistency = 0;
        let base_pixel = rgb.get_pixel(x, y);

        for dy in 0..size {
            for dx in 0..size {
                let pixel = rgb.get_pixel(x + dx, y + dy);
                let color_diff = self.color_difference(base_pixel, pixel);

                if color_diff < 30.0 {
                    pattern_consistency += 1;
                }
            }
        }

        let total_pixels = size * size;
        pattern_consistency > total_pixels / 2
    }

    fn analyze_dominant_colors(&self, rgb: &RgbImage) -> ColorAnalysis {
        let (width, height) = rgb.dimensions();
        let mut color_buckets = [0u32; 8]; // R, G, B, Yellow, Cyan, Magenta, White, Black
        let mut total_pixels = 0;

        for y in (0..height).step_by(8) {
            for x in (0..width).step_by(8) {
                total_pixels += 1;
                let pixel = rgb.get_pixel(x, y);
                let [r, g, b] = pixel.0;

                // Categorize into color buckets
                let brightness = (r as u16 + g as u16 + b as u16) / 3;

                if brightness < 50 {
                    color_buckets[7] += 1; // Black
                } else if brightness > 200 {
                    color_buckets[6] += 1; // White
                } else if r as u16 > g as u16 + 30 && r as u16 > b as u16 + 30 {
                    color_buckets[0] += 1; // Red
                } else if g as u16 > r as u16 + 30 && g as u16 > b as u16 + 30 {
                    color_buckets[1] += 1; // Green
                } else if b as u16 > r as u16 + 30 && b as u16 > g as u16 + 30 {
                    color_buckets[2] += 1; // Blue
                }
            }
        }

        let max_bucket = color_buckets.iter().max().unwrap_or(&0);
        let has_strong_theme = total_pixels > 0 && (*max_bucket as f32 / total_pixels as f32) > 0.4;

        ColorAnalysis { has_strong_theme }
    }

    fn count_horizontal_lines(&self, rgb: &RgbImage, width: u32, height: u32) -> u32 {
        let mut lines = 0;

        for y in (0..height).step_by(4) {
            let mut consecutive_similar = 0;
            let mut last_brightness = None;

            for x in (0..width).step_by(2) {
                let pixel = rgb.get_pixel(x, y);
                let brightness = (pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16) / 3;

                if let Some(last) = last_brightness {
                    if (brightness as i16 - last as i16).abs() < 20 {
                        consecutive_similar += 1;
                    } else {
                        consecutive_similar = 0;
                    }
                }

                last_brightness = Some(brightness);

                if consecutive_similar > width / 8 {
                    lines += 1;
                    break;
                }
            }
        }

        lines
    }

    fn count_vertical_lines(&self, rgb: &RgbImage, width: u32, height: u32) -> u32 {
        let mut lines = 0;

        for x in (0..width).step_by(4) {
            let mut consecutive_similar = 0;
            let mut last_brightness = None;

            for y in (0..height).step_by(2) {
                let pixel = rgb.get_pixel(x, y);
                let brightness = (pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16) / 3;

                if let Some(last) = last_brightness {
                    if (brightness as i16 - last as i16).abs() < 20 {
                        consecutive_similar += 1;
                    } else {
                        consecutive_similar = 0;
                    }
                }

                last_brightness = Some(brightness);

                if consecutive_similar > height / 8 {
                    lines += 1;
                    break;
                }
            }
        }

        lines
    }

    fn detect_rectangular_structures(&self, rgb: &RgbImage) -> bool {
        // Look for rectangular building/structure patterns
        let horizontal_lines = self.count_horizontal_lines(rgb, rgb.width(), rgb.height());
        let vertical_lines = self.count_vertical_lines(rgb, rgb.width(), rgb.height());

        horizontal_lines >= 2 && vertical_lines >= 2
    }

    fn detect_large_structures(&self, rgb: &RgbImage) -> bool {
        // Detect if structures take up significant portion of screen
        let structure_pixels = self.count_structure_pixels(rgb);
        let total_pixels = rgb.width() * rgb.height();

        (structure_pixels as f32 / total_pixels as f32) > 0.3
    }

    fn count_structure_variety(&self, rgb: &RgbImage) -> u32 {
        // Count different types of structures/patterns in the image
        let mut variety_score = 0;

        if self.detect_rectangular_structures(rgb) {
            variety_score += 1;
        }
        if self.detect_road_patterns(rgb) {
            variety_score += 1;
        }
        if self.detect_water_area(rgb) {
            variety_score += 1;
        }
        if self.detect_geometric_patterns(rgb) {
            variety_score += 1;
        }

        variety_score
    }

    fn count_structure_pixels(&self, rgb: &RgbImage) -> u32 {
        let (width, height) = rgb.dimensions();
        let mut structure_pixels = 0;

        for y in (0..height).step_by(4) {
            for x in (0..width).step_by(4) {
                if self.pixel_looks_like_structure(rgb, x, y) {
                    structure_pixels += 1;
                }
            }
        }

        structure_pixels
    }

    fn pixel_looks_like_structure(&self, rgb: &RgbImage, x: u32, y: u32) -> bool {
        let pixel = rgb.get_pixel(x, y);
        let [r, g, b] = pixel.0;

        // Structure pixels are typically not natural colors (not green grass, blue water)
        let is_natural_green = g as u16 > r as u16 + 20 && g as u16 > b as u16 + 10;
        let is_natural_blue = b as u16 > r as u16 + 20 && b as u16 > g as u16 + 10;

        !is_natural_green && !is_natural_blue
    }

    fn color_difference(&self, pixel1: &image::Rgb<u8>, pixel2: &image::Rgb<u8>) -> f32 {
        let r_diff = (pixel1[0] as i16 - pixel2[0] as i16).abs() as f32;
        let g_diff = (pixel1[1] as i16 - pixel2[1] as i16).abs() as f32;
        let b_diff = (pixel1[2] as i16 - pixel2[2] as i16).abs() as f32;

        (r_diff + g_diff + b_diff) / 3.0
    }

    fn detect_text_simple(&self, rgb_image: &RgbImage) -> bool {
        // Simple text detection: look for areas with high contrast
        let (width, height) = rgb_image.dimensions();

        let mut high_contrast_count = 0;
        let mut total_samples = 0;

        for y in (0..height).step_by(8) {
            for x in (0..width).step_by(8) {
                if x > 0 && y > 0 && x < width - 1 && y < height - 1 {
                    let current = rgb_image.get_pixel(x, y);
                    let left = rgb_image.get_pixel(x - 1, y);
                    let above = rgb_image.get_pixel(x, y - 1);

                    let current_brightness =
                        (current[0] as f32 + current[1] as f32 + current[2] as f32) / 3.0;
                    let left_brightness = (left[0] as f32 + left[1] as f32 + left[2] as f32) / 3.0;
                    let above_brightness =
                        (above[0] as f32 + above[1] as f32 + above[2] as f32) / 3.0;

                    if (current_brightness - left_brightness).abs() > 50.0
                        || (current_brightness - above_brightness).abs() > 50.0
                    {
                        high_contrast_count += 1;
                    }
                    total_samples += 1;
                }
            }
        }

        if total_samples == 0 {
            return false;
        }

        // If more than 20% of samples have high contrast, likely has text
        high_contrast_count as f32 / total_samples as f32 > 0.2
    }

    fn detect_menu_simple(&self, rgb_image: &RgbImage) -> bool {
        // Simple menu detection: look for rectangular patterns
        let (width, height) = rgb_image.dimensions();

        let mut menu_indicators = 0;

        for y in (0..height).step_by(16) {
            for x in (0..width).step_by(16) {
                if self.looks_like_menu_item(&rgb_image, x, y) {
                    menu_indicators += 1;
                }
            }
        }

        menu_indicators >= 2 // At least 2 menu-like items
    }

    fn looks_like_menu_item(&self, image: &RgbImage, x: u32, y: u32) -> bool {
        let size = 16;
        if x + size > image.width() || y + size > image.height() {
            return false;
        }

        // Precompute center brightness once
        let center = image.get_pixel(x + size / 2, y + size / 2);
        let center_brightness = (center[0] as f32 + center[1] as f32 + center[2] as f32) / 3.0;

        // Count border pixels that differ sufficiently from the center
        let mut border_pixels = 0u32;
        let mut high_contrast_border = 0u32;

        for dy in 0..size {
            for dx in 0..size {
                let is_border = dx == 0 || dx == size - 1 || dy == 0 || dy == size - 1;
                if !is_border {
                    continue;
                }

                let p = image.get_pixel(x + dx, y + dy);
                let pb = (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0;
                border_pixels += 1;
                if (center_brightness - pb).abs() >= 30.0 {
                    high_contrast_border += 1;
                }
            }
        }

        // Require a strong majority of border pixels to contrast with the center
        border_pixels > 0 && (high_contrast_border as f32 / border_pixels as f32) >= 0.7
    }

    fn detect_dialog_box_bottom(&self, rgb: &RgbImage) -> bool {
        // Look for a wide high-contrast band near the bottom
        let (w, h) = rgb.dimensions();
        if h < 32 || w < 64 {
            return false;
        }

        // Scan the bottom 20% of the image in horizontal stripes
        let start_y = (h as f32 * 0.8) as u32;
        let mut strong_rows = 0u32;
        let mut total_rows = 0u32;

        for y in (start_y..h).step_by(2) {
            total_rows += 1;
            // sample columns
            let mut transitions = 0u32;
            let mut last_brightness: Option<f32> = None;
            for x in (0..w).step_by(6) {
                let p = rgb.get_pixel(x, y);
                let b = (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0;
                if let Some(lb) = last_brightness {
                    if (b - lb).abs() > 35.0 {
                        transitions += 1;
                    }
                }
                last_brightness = Some(b);
            }
            if transitions > (w / 4) / 6 {
                // row has enough contrast transitions
                strong_rows += 1;
            }
        }

        // If enough strong rows found, likely a dialog box region
        total_rows > 0 && (strong_rows as f32 / total_rows as f32) > 0.3
    }
}

struct ColorAnalysis {
    has_strong_theme: bool,
}

impl Service<EnrichedFrame> for SceneAnnotationService {
    type Response = EnrichedFrame;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut enriched_frame: EnrichedFrame) -> Self::Future {
        let scene = self.detect_scene(&enriched_frame.image);

        // Debug logging to see what scenes are being detected
        tracing::info!("Scene detected: {:?}", scene);

        // Detect Pokemon Black specific state information
        let pokemon_state = self.analyze_pokemon_black_state(&enriched_frame.image, scene);

        if let Some(state) = &mut enriched_frame.state {
            // Update existing state with new detection
            state.scene = scene;
            state.location_type = pokemon_state.location_type;
            state.current_location = pokemon_state.current_location;
            state.in_tall_grass = pokemon_state.in_tall_grass;
            state.menu_cursor_position = pokemon_state.menu_cursor_position;
            state.battle_turn = pokemon_state.battle_turn;
            // Keep existing counts and progress if available
        } else {
            enriched_frame.state = Some(pokemon_state);
        }

        Box::pin(async move { Ok(enriched_frame) })
    }
}
