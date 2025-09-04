//! Detector for the menu cursor in Pokemon games.
use image::RgbImage;

use super::core::{
    DetectionContext, DetectionMetadata, DetectionResult, DetectionSignal, DetectionSignalType,
    ImageRegion, VisualDetector,
};

/// Detector for the menu cursor (often a hand icon).
pub struct MenuCursorDetector {
    /// The color of the cursor to look for (typically black or dark gray).
    pub cursor_color_threshold: u8,
    /// Minimum number of pixels that must match the cursor color.
    pub min_pixel_count: u32,
}

impl MenuCursorDetector {
    pub fn new() -> Self {
        Self {
            cursor_color_threshold: 50, // Very dark pixels
            min_pixel_count: 10,        // A small cluster of pixels for the cursor
        }
    }

    /// Checks if a pixel is likely part of the menu cursor.
    fn is_cursor_pixel(&self, r: u8, g: u8, b: u8) -> bool {
        let brightness = (r as u16 + g as u16 + b as u16) / 3;
        brightness < self.cursor_color_threshold as u16
    }
}

impl Default for MenuCursorDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualDetector for MenuCursorDetector {
    fn detect(&self, context: &DetectionContext) -> DetectionResult<Vec<DetectionSignal>> {
        let start_time = std::time::Instant::now();
        let mut signals = Vec::new();

        // Menus are typically on the bottom screen.
        let bottom_screen = ImageRegion::bottom_screen(context.dimensions.0, context.dimensions.1);
        let mut cursor_pixels = vec![];

        // The cursor is small, so we can't sample too aggressively.
        for y in (bottom_screen.y..bottom_screen.y + bottom_screen.height).step_by(1) {
            for x in (bottom_screen.x..bottom_screen.x + bottom_screen.width).step_by(1) {
                if let Some(pixel) = context.rgb.get_pixel_checked(x, y) {
                    if self.is_cursor_pixel(pixel.0[0], pixel.0[1], pixel.0[2]) {
                        cursor_pixels.push((x, y));
                    }
                }
            }
        }

        if cursor_pixels.len() > self.min_pixel_count as usize {
            // Find the average position of the cursor pixels to get a center point.
            let (sum_x, sum_y) = cursor_pixels
                .iter()
                .fold((0, 0), |(sx, sy), (px, py)| (sx + px, sy + py));
            let center_x = sum_x / cursor_pixels.len() as u32;
            let center_y = sum_y / cursor_pixels.len() as u32;

            signals.push(DetectionSignal {
                signal_type: DetectionSignalType::MenuCursor,
                confidence: 0.9,
                location: Some(ImageRegion::new(center_x, center_y, 1, 1)), // Point location
                metadata: DetectionMetadata::Position(center_x, center_y),
            });
        }

        let confidence = if signals.is_empty() { 0.0 } else { 0.9 };

        DetectionResult::new(
            signals,
            confidence,
            format!(
                "Menu cursor detection found {} pixels.",
                cursor_pixels.len()
            ),
        )
        .with_timing(start_time)
    }

    fn priority(&self) -> u8 {
        90 // High priority for menu navigation.
    }

    fn name(&self) -> &'static str {
        "MenuCursorDetector"
    }

    /// Should only run if a menu is present.
    fn can_process(&self, context: &DetectionContext) -> bool {
        context.has_signal(DetectionSignalType::BattleMenu)
            || context.has_signal(DetectionSignalType::MainMenu)
    }
}
