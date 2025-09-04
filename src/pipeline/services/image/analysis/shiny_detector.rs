//! Detector for shiny Pokemon animations.
use image::RgbImage;

use super::core::{
    DetectionContext, DetectionMetadata, DetectionResult, DetectionSignal, DetectionSignalType,
    ImageRegion, VisualDetector,
};

/// Detector for shiny Pokemon effects (sparkles, stars)
pub struct ShinyDetector {
    /// How bright a pixel needs to be relative to its neighbors to be considered a sparkle.
    pub sparkle_threshold: f32,
    /// The minimum number of sparkling pixels to trigger a detection.
    pub min_sparkle_count: u32,
}

impl ShinyDetector {
    pub fn new() -> Self {
        Self {
            sparkle_threshold: 2.5, // Pixel must be 2.5x brighter than neighbors
            min_sparkle_count: 10,  // Need at least 10 sparkling pixels
        }
    }

    /// Checks if a pixel is a "sparkle" by comparing its brightness to its neighbors.
    fn is_sparkle(&self, rgb: &RgbImage, x: u32, y: u32) -> bool {
        let pixel = match rgb.get_pixel_checked(x, y) {
            Some(p) => p,
            None => return false,
        };

        let [r, g, b] = pixel.0;
        // Using luminance to determine brightness
        let brightness = (0.2126 * r as f32) + (0.7152 * g as f32) + (0.0722 * b as f32);

        // Avoid division by zero and very dark pixels causing false positives
        if brightness < 30.0 {
            return false;
        }

        let mut neighbor_brightness_sum = 0.0;
        let mut neighbor_count = 0;

        // Check 8 neighbors in a 3x3 grid
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }

                if let Some(neighbor) =
                    rgb.get_pixel_checked((x as i32 + dx) as u32, (y as i32 + dy) as u32)
                {
                    let [nr, ng, nb] = neighbor.0;
                    neighbor_brightness_sum +=
                        (0.2126 * nr as f32) + (0.7152 * ng as f32) + (0.0722 * nb as f32);
                    neighbor_count += 1;
                }
            }
        }

        if neighbor_count > 0 {
            let avg_neighbor_brightness = neighbor_brightness_sum / neighbor_count as f32;
            if avg_neighbor_brightness > 0.0 {
                // If the pixel is significantly brighter than its neighbors, it's a sparkle.
                return brightness / avg_neighbor_brightness > self.sparkle_threshold;
            }
        }

        false
    }
}

impl Default for ShinyDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualDetector for ShinyDetector {
    fn detect(&self, context: &DetectionContext) -> DetectionResult<Vec<DetectionSignal>> {
        let start_time = std::time::Instant::now();
        let mut signals = Vec::new();

        // In Pokemon Black/White, the opponent Pokemon is on the top screen.
        let (width, height) = context.dimensions;
        let opponent_region = ImageRegion::top_screen(width, height);

        let mut sparkle_count = 0;
        let mut sparkle_locations = vec![];

        // Sample pixels in the opponent region to find sparkles.
        // A step of 2 should be fast enough.
        for y in (opponent_region.y..opponent_region.y + opponent_region.height).step_by(2) {
            for x in (opponent_region.x..opponent_region.x + opponent_region.width).step_by(2) {
                if self.is_sparkle(&context.rgb, x, y) {
                    sparkle_count += 1;
                    sparkle_locations.push((x, y));
                }
            }
        }

        if sparkle_count >= self.min_sparkle_count {
            signals.push(DetectionSignal {
                signal_type: DetectionSignalType::ShinyEffect,
                confidence: 0.95, // High confidence if we see enough sparkles
                location: Some(opponent_region),
                metadata: DetectionMetadata::Count(sparkle_count as usize),
            });
        }

        let confidence = if !signals.is_empty() { 0.95 } else { 0.05 };
        DetectionResult::new(
            signals,
            confidence,
            format!("Shiny detection found {} sparkles", sparkle_count),
        )
        .with_timing(start_time)
    }

    fn priority(&self) -> u8 {
        100 // Shiny detection is very high priority
    }

    fn name(&self) -> &'static str {
        "ShinyDetector"
    }

    /// Shiny detection should only run during a battle scene, after a Pokemon has appeared.
    fn can_process(&self, context: &DetectionContext) -> bool {
        context.has_signal(DetectionSignalType::HPBar)
    }
}
