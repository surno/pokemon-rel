use crate::pipeline::services::learning::smart_action_service::{GameSituation, UrgencyLevel};
/// High-performance situation analyzer that avoids expensive image processing
use crate::pipeline::{EnrichedFrame, Scene};
use image::{DynamicImage, RgbImage};
use std::sync::Arc;

/// Fast situation analyzer that uses caching and avoids redundant processing
pub struct FastSituationAnalyzer {
    /// Cache RGB conversions to avoid repeated expensive operations
    rgb_cache: Option<(Arc<DynamicImage>, Arc<RgbImage>)>,
    /// Cache analysis results for identical images
    analysis_cache: Option<(u64, GameSituation)>,
    /// Skip expensive analysis when scene is already known with high confidence
    skip_expensive_analysis: bool,
}

impl FastSituationAnalyzer {
    pub fn new() -> Self {
        Self {
            rgb_cache: None,
            analysis_cache: None,
            skip_expensive_analysis: true,
        }
    }

    pub fn with_expensive_analysis(mut self, enabled: bool) -> Self {
        self.skip_expensive_analysis = !enabled;
        self
    }

    /// Fast situation analysis with aggressive caching
    pub fn analyze_situation_fast(&mut self, frame: &EnrichedFrame) -> GameSituation {
        // Fast path: Use scene from state if available and confident
        if let Some(state) = &frame.state {
            if state.scene != Scene::Unknown {
                return self.analyze_from_known_scene(state.scene, frame);
            }
        }

        // Check if we can reuse cached RGB conversion
        let rgb = self.get_or_create_rgb(&frame.image);

        // Quick cache check using image pointer comparison
        let image_ptr = Arc::as_ptr(&frame.image) as u64;
        if let Some((cached_ptr, cached_situation)) = &self.analysis_cache {
            if *cached_ptr == image_ptr {
                return cached_situation.clone();
            }
        }

        // Perform fast analysis
        let situation = self.fast_analyze(&rgb, frame);

        // Cache the result
        self.analysis_cache = Some((image_ptr, situation.clone()));

        situation
    }

    /// Get RGB image from cache or create it
    fn get_or_create_rgb(&mut self, image: &Arc<DynamicImage>) -> Arc<RgbImage> {
        // Check if we can reuse cached RGB
        if let Some((cached_image, cached_rgb)) = &self.rgb_cache {
            if Arc::ptr_eq(cached_image, image) {
                return Arc::clone(cached_rgb);
            }
        }

        // Create new RGB and cache it
        let rgb = Arc::new(image.to_rgb8());
        self.rgb_cache = Some((Arc::clone(image), Arc::clone(&rgb)));
        rgb
    }

    /// Fast analysis using known scene information
    fn analyze_from_known_scene(&self, scene: Scene, _frame: &EnrichedFrame) -> GameSituation {
        let (has_text, has_menu, in_dialog, urgency) = match scene {
            Scene::Battle => (true, true, false, UrgencyLevel::High),
            Scene::MainMenu => (false, true, false, UrgencyLevel::Medium),
            Scene::Intro => (true, false, true, UrgencyLevel::Low),
            Scene::Overworld => (false, false, false, UrgencyLevel::Low),
            Scene::NameCreation => (true, true, false, UrgencyLevel::Medium),
            Scene::Unknown => (false, false, false, UrgencyLevel::Low),
        };

        GameSituation {
            scene,
            has_text,
            has_menu,
            has_buttons: has_menu,
            in_dialog,
            cursor_row: None, // Skip expensive cursor detection
            dominant_colors: self.get_cached_colors(scene),
            urgency_level: urgency,
        }
    }

    /// Fast analysis with minimal image processing
    fn fast_analyze(&self, rgb: &RgbImage, frame: &EnrichedFrame) -> GameSituation {
        if self.skip_expensive_analysis {
            // Ultra-fast path: Use only basic heuristics
            return self.heuristic_analysis(frame);
        }

        // Reduced sampling for speed (check every 8th pixel instead of every pixel)
        let (width, height) = rgb.dimensions();
        let mut text_pixels = 0;
        let mut menu_pixels = 0;
        let mut total_sampled = 0;

        // Sample only 1/64th of the image for speed
        for y in (0..height).step_by(8) {
            for x in (0..width).step_by(8) {
                if let Some(pixel) = rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    let brightness = (r as u16 + g as u16 + b as u16) / 3;

                    // Fast text detection (high contrast)
                    if brightness < 50 || brightness > 200 {
                        text_pixels += 1;
                    }

                    // Fast menu detection (specific colors)
                    if brightness > 150 && r > 100 && g > 100 && b > 100 {
                        menu_pixels += 1;
                    }

                    total_sampled += 1;
                }
            }
        }

        // Calculate ratios
        let text_ratio = if total_sampled > 0 {
            text_pixels as f32 / total_sampled as f32
        } else {
            0.0
        };
        let menu_ratio = if total_sampled > 0 {
            menu_pixels as f32 / total_sampled as f32
        } else {
            0.0
        };

        // Determine scene based on ratios
        let scene = if text_ratio > 0.3 && menu_ratio > 0.1 {
            Scene::Battle
        } else if menu_ratio > 0.2 {
            Scene::MainMenu
        } else if text_ratio > 0.25 && menu_ratio > 0.05 {
            Scene::NameCreation // Character grid has both text and menu elements
        } else if text_ratio > 0.2 {
            Scene::Intro
        } else {
            Scene::Overworld
        };

        GameSituation {
            scene,
            has_text: text_ratio > 0.15,
            has_menu: menu_ratio > 0.1,
            has_buttons: menu_ratio > 0.1,
            in_dialog: text_ratio > 0.25,
            cursor_row: None, // Skip expensive cursor detection
            dominant_colors: self.get_cached_colors(scene),
            urgency_level: self.determine_urgency(scene),
        }
    }

    /// Ultra-fast heuristic analysis (no image processing)
    fn heuristic_analysis(&self, frame: &EnrichedFrame) -> GameSituation {
        // Use frame metadata if available
        let scene = frame
            .state
            .as_ref()
            .map(|s| s.scene)
            .unwrap_or(Scene::Overworld);

        GameSituation {
            scene,
            has_text: false,
            has_menu: false,
            has_buttons: false,
            in_dialog: false,
            cursor_row: None,
            dominant_colors: self.get_cached_colors(scene),
            urgency_level: self.determine_urgency(scene),
        }
    }

    /// Get pre-computed dominant colors for each scene type
    fn get_cached_colors(&self, scene: Scene) -> Vec<String> {
        match scene {
            Scene::Battle => vec!["red".to_string(), "green".to_string(), "blue".to_string()],
            Scene::MainMenu => vec!["blue".to_string(), "white".to_string()],
            Scene::Intro => vec!["black".to_string(), "white".to_string()],
            Scene::Overworld => vec!["green".to_string(), "brown".to_string()],
            Scene::NameCreation => {
                vec!["blue".to_string(), "white".to_string(), "black".to_string()]
            }
            Scene::Unknown => vec!["gray".to_string()],
        }
    }

    /// Determine urgency level based on scene
    fn determine_urgency(&self, scene: Scene) -> UrgencyLevel {
        match scene {
            Scene::Battle => UrgencyLevel::High,
            Scene::MainMenu => UrgencyLevel::Medium,
            Scene::Intro => UrgencyLevel::Low,
            Scene::Overworld => UrgencyLevel::Low,
            Scene::NameCreation => UrgencyLevel::Medium,
            Scene::Unknown => UrgencyLevel::Low,
        }
    }

    /// Clear caches to free memory
    pub fn clear_cache(&mut self) {
        self.rgb_cache = None;
        self.analysis_cache = None;
    }

    /// Get cache statistics for debugging
    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            has_rgb_cache: self.rgb_cache.is_some(),
            has_analysis_cache: self.analysis_cache.is_some(),
            skip_expensive_analysis: self.skip_expensive_analysis,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub has_rgb_cache: bool,
    pub has_analysis_cache: bool,
    pub skip_expensive_analysis: bool,
}

impl Default for FastSituationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
