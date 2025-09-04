use crate::{
    error::AppError,
    pipeline::{EnrichedFrame, Scene, State},
};
use image::{DynamicImage, RgbImage};
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
        // Heuristic detection: analyze image features
        let has_text = self.detect_text_simple(frame);
        let has_menu = self.detect_menu_simple(frame);
        let has_dialog = self.detect_dialog_box_bottom(frame);

        if has_text && has_menu {
            // Typical battle UI presents both menu and text
            return Scene::Battle;
        }
        if has_menu {
            return Scene::MainMenu;
        }
        if has_text || has_dialog {
            return Scene::Intro;
        }
        Scene::Unknown
    }

    fn detect_text_simple(&self, image: &DynamicImage) -> bool {
        // Simple text detection: look for areas with high contrast
        let rgb_image = image.to_rgb8();
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

    fn detect_menu_simple(&self, image: &DynamicImage) -> bool {
        // Simple menu detection: look for rectangular patterns
        let rgb_image = image.to_rgb8();
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

    fn detect_dialog_box_bottom(&self, image: &DynamicImage) -> bool {
        // Look for a wide high-contrast band near the bottom
        let rgb = image.to_rgb8();
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
            for x in (0..w).step_by(4) {
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
