//! Detector for Pokemon appearance and species.
use image::RgbImage;

use super::core::{
    DetectionContext, DetectionMetadata, DetectionResult, DetectionSignal, DetectionSignalType,
    ImageRegion, VisualDetector,
};

/// Detector for wild Pokemon encounters.
pub struct PokemonDetector {
    /// A threshold for how much of the screen must change to be considered an encounter animation.
    pub screen_change_threshold: f32,
}

impl PokemonDetector {
    pub fn new() -> Self {
        Self {
            screen_change_threshold: 0.75, // 75% of pixels must be different for encounter animation
        }
    }

    /// Detects the "flash" animation of a wild Pokemon encounter.
    /// This is a simplified check that looks for rapid, widespread brightness changes.
    fn detect_encounter_flash(&self, rgb: &RgbImage) -> (bool, f32) {
        let (width, height) = rgb.dimensions();
        let mut changed_pixels = 0;
        let total_pixels = width * height;

        // We're looking for a screen that is almost all white or all black, which is common
        // in encounter flash animations.
        for y in (0..height).step_by(4) {
            for x in (0..width).step_by(4) {
                if let Some(pixel) = rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    let brightness = (r as u16 + g as u16 + b as u16) / 3;
                    if brightness > 230 || brightness < 25 {
                        changed_pixels += 1;
                    }
                }
            }
        }
        let change_ratio = changed_pixels as f32 / (total_pixels / 16) as f32;
        (change_ratio > self.screen_change_threshold, change_ratio)
    }
}

impl Default for PokemonDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualDetector for PokemonDetector {
    fn detect(&self, context: &DetectionContext) -> DetectionResult<Vec<DetectionSignal>> {
        let start_time = std::time::Instant::now();
        let mut signals = Vec::new();

        let (is_flash, confidence) = self.detect_encounter_flash(&context.rgb);

        if is_flash {
            signals.push(DetectionSignal {
                signal_type: DetectionSignalType::WildPokemonEncounter,
                confidence,
                location: None,
                metadata: DetectionMetadata::None,
            });
        }
        // NOTE: Pokemon species detection would be a much more complex task, likely
        // requiring machine learning or sophisticated template matching. This detector
        // will only focus on the encounter event itself for now.

        let result_confidence = if signals.is_empty() { 0.0 } else { confidence };

        DetectionResult::new(
            signals,
            result_confidence,
            format!(
                "Pokemon encounter detection. Flash confidence: {:.2}",
                confidence
            ),
        )
        .with_timing(start_time)
    }

    fn priority(&self) -> u8 {
        95 // High priority, as it's a major state change.
    }

    fn name(&self) -> &'static str {
        "PokemonDetector"
    }

    /// This detector can run at any time, but it's most relevant in the overworld.
    fn can_process(&self, context: &DetectionContext) -> bool {
        !context.has_signal(DetectionSignalType::BattleMenu)
            && !context.has_signal(DetectionSignalType::HPBar)
    }
}
