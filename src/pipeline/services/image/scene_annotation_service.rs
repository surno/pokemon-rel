use crate::{
    error::AppError,
    pipeline::{EnrichedFrame, Scene, State},
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

    fn detect_scene(&self, frame: &DynamicImage) -> Scene {
        // Pokemon-specific scene detection
        let rgb = frame.to_rgb8();
        let (width, height) = rgb.dimensions();

        // Check for battle scene indicators
        if self.detect_battle_scene(&rgb) {
            return Scene::Battle;
        }

        // Check for main menu (title screen, menu options)
        if self.detect_main_menu(&rgb) {
            return Scene::MainMenu;
        }

        // Check for intro/dialog scenes
        if self.detect_intro_dialog(&rgb) {
            return Scene::Intro;
        }

        // If we can't identify the scene, it's likely overworld gameplay
        // In Pokemon, most gameplay happens in the overworld
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

                // Green HP bar detection
                if g > 150 && g > r + 30 && g > b + 30 {
                    consecutive_green += 1;
                    consecutive_red = 0;
                }
                // Red HP bar detection (low HP)
                else if r > 150 && r > g + 30 && r > b + 30 {
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

impl Service<EnrichedFrame> for SceneAnnotationService {
    type Response = EnrichedFrame;
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut enriched_frame: EnrichedFrame) -> Self::Future {
        let scene = self.detect_scene(&enriched_frame.image);

        if let Some(state) = &mut enriched_frame.state {
            state.scene = scene;
        } else {
            enriched_frame.state = Some(State {
                scene,
                player_position: (0.0, 0.0),
                pokemon_count: 0,
            });
        }

        Box::pin(async move { Ok(enriched_frame) })
    }
}
