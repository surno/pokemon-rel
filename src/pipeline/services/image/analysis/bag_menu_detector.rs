//! Detector for the Pokemon Bag Menu.
use image::RgbImage;

use super::core::{
    DetectionContext, DetectionMetadata, DetectionResult, DetectionSignal, DetectionSignalType,
    ImageRegion, VisualDetector,
};

/// Detector for the bag menu screen.
pub struct BagMenuDetector {
    /// Color similarity threshold for detecting the bag's characteristic brown/orange color.
    pub color_threshold: u8,
    /// Minimum percentage of the screen that must match the bag color.
    pub area_threshold: f32,
}

impl BagMenuDetector {
    pub fn new() -> Self {
        Self {
            color_threshold: 40,
            area_threshold: 0.50, // 50% of the top screen should have the bag color
        }
    }

    /// Checks if a pixel matches the characteristic color of the bag menu.
    fn is_bag_color(&self, r: u8, g: u8, b: u8) -> bool {
        // The bag in Pokemon Black has a brownish/orangish color theme.
        // R > G > B, with R being significantly higher than B.
        let is_brownish = r > 150 && g > 80 && g < 150 && b < 100;
        let r_g_diff = r.saturating_sub(g);
        let g_b_diff = g.saturating_sub(b);

        is_brownish && r_g_diff > self.color_threshold && g_b_diff > 20
    }
}

impl Default for BagMenuDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualDetector for BagMenuDetector {
    fn detect(&self, context: &DetectionContext) -> DetectionResult<Vec<DetectionSignal>> {
        let start_time = std::time::Instant::now();
        let mut signals = Vec::new();

        // The bag menu primarily occupies the top screen.
        let top_screen = ImageRegion::top_screen(context.dimensions.0, context.dimensions.1);
        let mut bag_color_pixels = 0;
        let total_pixels = top_screen.area();

        for y in (top_screen.y..top_screen.y + top_screen.height).step_by(4) {
            for x in (top_screen.x..top_screen.x + top_screen.width).step_by(4) {
                if let Some(pixel) = context.rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    if self.is_bag_color(r, g, b) {
                        bag_color_pixels += 1;
                    }
                }
            }
        }

        let area_ratio = bag_color_pixels as f32 / (total_pixels / 16) as f32;

        if area_ratio > self.area_threshold {
            signals.push(DetectionSignal {
                signal_type: DetectionSignalType::BagMenu,
                confidence: area_ratio.min(1.0),
                location: Some(top_screen),
                metadata: DetectionMetadata::None,
            });
        }

        let confidence = if signals.is_empty() { 0.0 } else { area_ratio };

        DetectionResult::new(
            signals,
            confidence,
            format!(
                "Bag Menu detection with {:.2}% area coverage",
                area_ratio * 100.0
            ),
        )
        .with_timing(start_time)
    }

    fn priority(&self) -> u8 {
        85 // High priority, as it's a key UI screen.
    }

    fn name(&self) -> &'static str {
        "BagMenuDetector"
    }

    /// Should not run during a battle.
    fn can_process(&self, context: &DetectionContext) -> bool {
        !context.has_signal(DetectionSignalType::HPBar)
    }
}
