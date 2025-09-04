/// Micro-detectors for specific visual elements using Template Method pattern
use super::core::{
    DetectionContext, DetectionMetadata, DetectionResult, DetectionSignal, DetectionSignalType,
    ImageRegion, VisualDetector,
};
use image::RgbImage;

/// Template Method pattern for common image analysis operations
pub trait ImageAnalyzer {
    fn analyze_region(&self, rgb: &RgbImage, region: ImageRegion) -> f32;
    fn get_threshold(&self) -> f32;
    fn get_signal_type(&self) -> DetectionSignalType;

    /// Template method - common detection logic
    fn detect_in_region(&self, rgb: &RgbImage, region: ImageRegion) -> DetectionResult<bool> {
        let start_time = std::time::Instant::now();
        let score = self.analyze_region(rgb, region);
        let detected = score > self.get_threshold();
        let confidence = if detected { score } else { 1.0 - score };

        DetectionResult::new(
            detected,
            confidence,
            format!(
                "{} detection in region {:?}",
                self.get_signal_type_name(),
                region
            ),
        )
        .with_timing(start_time)
    }

    fn get_signal_type_name(&self) -> &'static str {
        match self.get_signal_type() {
            DetectionSignalType::HPBar => "HP Bar",
            DetectionSignalType::BattleMenu => "Battle Menu",
            DetectionSignalType::MainMenu => "Main Menu",
            DetectionSignalType::DialogBox => "Dialog Box",
            DetectionSignalType::TextBlock => "Text Block",
            DetectionSignalType::MenuCursor => "Menu Cursor",
            DetectionSignalType::TallGrass => "Tall Grass",
            DetectionSignalType::Water => "Water",
            DetectionSignalType::Indoor => "Indoor",
            DetectionSignalType::Outdoor => "Outdoor",
            DetectionSignalType::PokemonCenter => "Pokemon Center",
            DetectionSignalType::Gym => "Gym",
            DetectionSignalType::Cave => "Cave",
            DetectionSignalType::City => "City",
            DetectionSignalType::Town => "Town",
            DetectionSignalType::Route => "Route",
            DetectionSignalType::Building => "Building",
            DetectionSignalType::BattleTurn => "Battle Turn",
            DetectionSignalType::MenuOption => "Menu Option",
            DetectionSignalType::PlayerPosition => "Player Position",
        }
    }
}

/// HP Bar detector using Template Method pattern
pub struct HPBarDetector {
    green_threshold: u8,
    red_threshold: u8,
    min_bar_length: u32,
}

impl HPBarDetector {
    pub fn new() -> Self {
        Self {
            green_threshold: 150,
            red_threshold: 150,
            min_bar_length: 16,
        }
    }

    pub fn with_thresholds(mut self, green: u8, red: u8) -> Self {
        self.green_threshold = green;
        self.red_threshold = red;
        self
    }
}

impl ImageAnalyzer for HPBarDetector {
    fn analyze_region(&self, rgb: &RgbImage, region: ImageRegion) -> f32 {
        let mut max_bar_length = 0;
        let mut total_bar_pixels = 0;

        for y in region.y..(region.y + region.height) {
            let mut consecutive_green = 0;
            let mut consecutive_red = 0;

            for x in region.x..(region.x + region.width) {
                if let Some(pixel) = rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;

                    // Green HP bar detection
                    if g > self.green_threshold
                        && g as u16 > r as u16 + 30
                        && g as u16 > b as u16 + 30
                    {
                        consecutive_green += 1;
                        consecutive_red = 0;
                        total_bar_pixels += 1;
                    }
                    // Red HP bar detection (low HP)
                    else if r > self.red_threshold
                        && r as u16 > g as u16 + 30
                        && r as u16 > b as u16 + 30
                    {
                        consecutive_red += 1;
                        consecutive_green = 0;
                        total_bar_pixels += 1;
                    } else {
                        max_bar_length = max_bar_length.max(consecutive_green).max(consecutive_red);
                        consecutive_green = 0;
                        consecutive_red = 0;
                    }
                }
            }
            max_bar_length = max_bar_length.max(consecutive_green).max(consecutive_red);
        }

        // Score based on longest bar found and total bar pixels
        let length_score = (max_bar_length as f32 / region.width as f32).min(1.0);
        let density_score = (total_bar_pixels as f32 / region.area() as f32).min(1.0);

        (length_score + density_score) / 2.0
    }

    fn get_threshold(&self) -> f32 {
        0.3 // 30% confidence threshold
    }

    fn get_signal_type(&self) -> DetectionSignalType {
        DetectionSignalType::HPBar
    }
}

impl VisualDetector for HPBarDetector {
    fn detect(&self, context: &DetectionContext) -> DetectionResult<Vec<DetectionSignal>> {
        let start_time = std::time::Instant::now();

        // Focus on top quarter where HP bars typically appear
        let region = ImageRegion::top_quarter(context.dimensions.0, context.dimensions.1);
        let detection = self.detect_in_region(&context.rgb, region);

        let signals = if detection.result {
            vec![DetectionSignal {
                signal_type: self.get_signal_type(),
                confidence: detection.confidence,
                location: Some(region),
                metadata: DetectionMetadata::None,
            }]
        } else {
            vec![]
        };

        DetectionResult::new(signals, detection.confidence, detection.reasoning)
            .with_timing(start_time)
    }

    fn priority(&self) -> u8 {
        90 // High priority - HP bars are strong battle indicators
    }

    fn name(&self) -> &'static str {
        "HPBarDetector"
    }

    fn can_process(&self, _context: &DetectionContext) -> bool {
        true // Can always process
    }
}

/// Text detector for dialog boxes and menus
pub struct TextDetector {
    contrast_threshold: u8,
    min_text_density: f32,
}

impl TextDetector {
    pub fn new() -> Self {
        Self {
            contrast_threshold: 100,
            min_text_density: 0.15,
        }
    }

    pub fn with_threshold(mut self, threshold: u8) -> Self {
        self.contrast_threshold = threshold;
        self
    }
}

impl ImageAnalyzer for TextDetector {
    fn analyze_region(&self, rgb: &RgbImage, region: ImageRegion) -> f32 {
        let mut text_pixels = 0;
        let total_pixels = region.area();

        for y in (region.y..(region.y + region.height)).step_by(2) {
            for x in (region.x..(region.x + region.width)).step_by(2) {
                if self.pixel_has_text_contrast(rgb, x, y) {
                    text_pixels += 1;
                }
            }
        }

        text_pixels as f32 / (total_pixels / 4) as f32 // Adjust for step sampling
    }

    fn get_threshold(&self) -> f32 {
        self.min_text_density
    }

    fn get_signal_type(&self) -> DetectionSignalType {
        DetectionSignalType::TextBlock
    }
}

impl TextDetector {
    fn pixel_has_text_contrast(&self, rgb: &RgbImage, x: u32, y: u32) -> bool {
        if let Some(pixel) = rgb.get_pixel_checked(x, y) {
            let [r, g, b] = pixel.0;
            let brightness = (r as u16 + g as u16 + b as u16) / 3;

            // Check neighboring pixels for contrast
            let mut contrast_found = false;
            for dy in 0..=1 {
                for dx in 0..=1 {
                    if let Some(neighbor) = rgb.get_pixel_checked(x + dx, y + dy) {
                        let [nr, ng, nb] = neighbor.0;
                        let neighbor_brightness = (nr as u16 + ng as u16 + nb as u16) / 3;
                        let contrast = brightness.abs_diff(neighbor_brightness);

                        if contrast > self.contrast_threshold as u16 {
                            contrast_found = true;
                            break;
                        }
                    }
                }
                if contrast_found {
                    break;
                }
            }
            contrast_found
        } else {
            false
        }
    }
}

impl VisualDetector for TextDetector {
    fn detect(&self, context: &DetectionContext) -> DetectionResult<Vec<DetectionSignal>> {
        let start_time = std::time::Instant::now();
        let mut signals = Vec::new();

        // Check multiple regions for text
        let regions = vec![
            ImageRegion::full_image(context.dimensions.0, context.dimensions.1),
            ImageRegion::bottom_quarter(context.dimensions.0, context.dimensions.1),
            ImageRegion::center_half(context.dimensions.0, context.dimensions.1),
        ];

        for region in regions {
            let detection = self.detect_in_region(&context.rgb, region);
            if detection.result {
                signals.push(DetectionSignal {
                    signal_type: self.get_signal_type(),
                    confidence: detection.confidence,
                    location: Some(region),
                    metadata: DetectionMetadata::None,
                });
            }
        }

        let signal_count = signals.len();
        let overall_confidence = signals.iter().map(|s| s.confidence).fold(0.0, f32::max);

        DetectionResult::new(
            signals,
            overall_confidence,
            format!("Text detection found {} regions", signal_count),
        )
        .with_timing(start_time)
    }

    fn priority(&self) -> u8 {
        70 // Medium-high priority
    }

    fn name(&self) -> &'static str {
        "TextDetector"
    }

    fn can_process(&self, _context: &DetectionContext) -> bool {
        true
    }
}

/// Menu detector for battle menus and main menus
pub struct MenuDetector {
    min_menu_boxes: usize,
    box_size_threshold: u32,
}

impl MenuDetector {
    pub fn new() -> Self {
        Self {
            min_menu_boxes: 2,
            box_size_threshold: 16,
        }
    }
}

impl ImageAnalyzer for MenuDetector {
    fn analyze_region(&self, rgb: &RgbImage, region: ImageRegion) -> f32 {
        let mut menu_boxes = 0;

        for y in (region.y..(region.y + region.height)).step_by(8) {
            for x in (region.x..(region.x + region.width)).step_by(16) {
                if self.detect_menu_box(rgb, x, y, 32, 16) {
                    menu_boxes += 1;
                }
            }
        }

        menu_boxes as f32 / self.min_menu_boxes as f32
    }

    fn get_threshold(&self) -> f32 {
        1.0 // Need at least min_menu_boxes
    }

    fn get_signal_type(&self) -> DetectionSignalType {
        DetectionSignalType::BattleMenu
    }
}

impl MenuDetector {
    fn detect_menu_box(&self, rgb: &RgbImage, x: u32, y: u32, w: u32, h: u32) -> bool {
        // Check if this region looks like a menu box (simplified logic)
        let end_x = (x + w).min(rgb.width());
        let end_y = (y + h).min(rgb.height());

        let mut border_pixels = 0;
        let mut total_checked = 0;

        // Check top and bottom borders
        for check_x in x..end_x {
            if let Some(_top_pixel) = rgb.get_pixel_checked(check_x, y) {
                if self.pixel_looks_like_border(rgb, check_x, y) {
                    border_pixels += 1;
                }
                total_checked += 1;
            }
            if let Some(_bottom_pixel) = rgb.get_pixel_checked(check_x, end_y.saturating_sub(1)) {
                if self.pixel_looks_like_border(rgb, check_x, end_y.saturating_sub(1)) {
                    border_pixels += 1;
                }
                total_checked += 1;
            }
        }

        // Check left and right borders
        for check_y in y..end_y {
            if let Some(_left_pixel) = rgb.get_pixel_checked(x, check_y) {
                if self.pixel_looks_like_border(rgb, x, check_y) {
                    border_pixels += 1;
                }
                total_checked += 1;
            }
            if let Some(_right_pixel) = rgb.get_pixel_checked(end_x.saturating_sub(1), check_y) {
                if self.pixel_looks_like_border(rgb, end_x.saturating_sub(1), check_y) {
                    border_pixels += 1;
                }
                total_checked += 1;
            }
        }

        // If enough border pixels found, likely a menu box
        total_checked > 0 && border_pixels as f32 / total_checked as f32 > 0.3
    }

    fn pixel_looks_like_border(&self, rgb: &RgbImage, x: u32, y: u32) -> bool {
        if let Some(pixel) = rgb.get_pixel_checked(x, y) {
            let [r, g, b] = pixel.0;
            // Menu borders are typically dark or high contrast
            let brightness = (r as u16 + g as u16 + b as u16) / 3;
            brightness < 50 || brightness > 200 // Very dark or very light
        } else {
            false
        }
    }
}

impl VisualDetector for MenuDetector {
    fn detect(&self, context: &DetectionContext) -> DetectionResult<Vec<DetectionSignal>> {
        let start_time = std::time::Instant::now();

        // Check bottom quarter for battle menus
        let bottom_region = ImageRegion::bottom_quarter(context.dimensions.0, context.dimensions.1);
        let detection = self.detect_in_region(&context.rgb, bottom_region);

        let signals = if detection.result {
            vec![DetectionSignal {
                signal_type: self.get_signal_type(),
                confidence: detection.confidence,
                location: Some(bottom_region),
                metadata: DetectionMetadata::None,
            }]
        } else {
            vec![]
        };

        DetectionResult::new(signals, detection.confidence, detection.reasoning)
            .with_timing(start_time)
    }

    fn priority(&self) -> u8 {
        80 // High priority - menus are strong scene indicators
    }

    fn name(&self) -> &'static str {
        "MenuDetector"
    }

    fn can_process(&self, _context: &DetectionContext) -> bool {
        true
    }
}

/// Location detector for different Pokemon locations
pub struct LocationDetector {
    confidence_threshold: f32,
}

impl LocationDetector {
    pub fn new() -> Self {
        Self {
            confidence_threshold: 0.6,
        }
    }
}

impl VisualDetector for LocationDetector {
    fn detect(&self, context: &DetectionContext) -> DetectionResult<Vec<DetectionSignal>> {
        let start_time = std::time::Instant::now();
        let mut signals = Vec::new();

        // Check for various location types
        if self.detect_pokemon_center(&context.rgb) {
            signals.push(DetectionSignal {
                signal_type: DetectionSignalType::PokemonCenter,
                confidence: 0.8,
                location: None,
                metadata: DetectionMetadata::None,
            });
        }

        if self.detect_gym(&context.rgb) {
            signals.push(DetectionSignal {
                signal_type: DetectionSignalType::Gym,
                confidence: 0.7,
                location: None,
                metadata: DetectionMetadata::None,
            });
        }

        if self.detect_cave(&context.rgb) {
            signals.push(DetectionSignal {
                signal_type: DetectionSignalType::Cave,
                confidence: 0.75,
                location: None,
                metadata: DetectionMetadata::None,
            });
        }

        let signal_count = signals.len();
        let overall_confidence = signals.iter().map(|s| s.confidence).fold(0.0, f32::max);

        DetectionResult::new(
            signals,
            overall_confidence,
            format!("Location detection found {} signals", signal_count),
        )
        .with_timing(start_time)
    }

    fn priority(&self) -> u8 {
        50 // Medium priority
    }

    fn name(&self) -> &'static str {
        "LocationDetector"
    }

    fn can_process(&self, context: &DetectionContext) -> bool {
        // Only process if we don't already have strong scene signals
        !context.has_signal(DetectionSignalType::HPBar)
            && !context.has_signal(DetectionSignalType::BattleMenu)
    }
}

impl LocationDetector {
    fn detect_pokemon_center(&self, rgb: &RgbImage) -> bool {
        // Look for characteristic Pokemon Center colors (pink/red healing machine)
        let (width, height) = rgb.dimensions();
        let mut pink_pixels = 0;

        for y in (0..height).step_by(4) {
            for x in (0..width).step_by(4) {
                if let Some(pixel) = rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    // Pink/red color detection for healing machine
                    if r > 180 && g < 150 && b > 100 && r > b {
                        pink_pixels += 1;
                    }
                }
            }
        }

        pink_pixels > (width * height) / 2000 // Threshold for Pokemon Center
    }

    fn detect_gym(&self, rgb: &RgbImage) -> bool {
        // Gyms typically have distinctive geometric patterns and colors
        self.detect_geometric_patterns(rgb) && self.detect_indoor_lighting(rgb)
    }

    fn detect_cave(&self, rgb: &RgbImage) -> bool {
        // Caves typically have dark colors and rock-like textures
        let (width, height) = rgb.dimensions();
        let mut dark_pixels = 0;

        for y in (0..height).step_by(4) {
            for x in (0..width).step_by(4) {
                if let Some(pixel) = rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    let brightness = (r as u16 + g as u16 + b as u16) / 3;
                    if brightness < 80 {
                        dark_pixels += 1;
                    }
                }
            }
        }

        dark_pixels > (width * height) / 1000 // Threshold for cave darkness
    }

    fn detect_geometric_patterns(&self, rgb: &RgbImage) -> bool {
        // Simplified geometric pattern detection
        let (width, height) = rgb.dimensions();
        let mut pattern_score = 0;

        for y in (0..height).step_by(8) {
            for x in (0..width).step_by(8) {
                if self.has_rectangular_pattern(rgb, x, y) {
                    pattern_score += 1;
                }
            }
        }

        pattern_score > 5 // Threshold for geometric patterns
    }

    fn detect_indoor_lighting(&self, rgb: &RgbImage) -> bool {
        // Indoor areas typically have more uniform lighting
        let (width, height) = rgb.dimensions();
        let mut uniform_regions = 0;

        for y in (0..height).step_by(16) {
            for x in (0..width).step_by(16) {
                if self.region_has_uniform_lighting(rgb, x, y, 16, 16) {
                    uniform_regions += 1;
                }
            }
        }

        uniform_regions > (width * height) / 4000 // Threshold for indoor lighting
    }

    fn has_rectangular_pattern(&self, rgb: &RgbImage, x: u32, y: u32) -> bool {
        // Simple rectangular pattern detection
        if let Some(pixel) = rgb.get_pixel_checked(x, y) {
            let [r, g, b] = pixel.0;
            // Look for edges or borders
            let brightness = (r as u16 + g as u16 + b as u16) / 3;
            brightness < 50 || brightness > 200
        } else {
            false
        }
    }

    fn region_has_uniform_lighting(&self, rgb: &RgbImage, x: u32, y: u32, w: u32, h: u32) -> bool {
        let mut total_brightness = 0u32;
        let mut pixel_count = 0u32;

        for check_y in y..(y + h).min(rgb.height()) {
            for check_x in x..(x + w).min(rgb.width()) {
                if let Some(pixel) = rgb.get_pixel_checked(check_x, check_y) {
                    let [r, g, b] = pixel.0;
                    total_brightness += r as u32 + g as u32 + b as u32;
                    pixel_count += 1;
                }
            }
        }

        if pixel_count > 0 {
            let avg_brightness = total_brightness / pixel_count;
            // Check if variance is low (uniform lighting)
            let mut variance_sum = 0u32;

            for check_y in y..(y + h).min(rgb.height()) {
                for check_x in x..(x + w).min(rgb.width()) {
                    if let Some(pixel) = rgb.get_pixel_checked(check_x, check_y) {
                        let [r, g, b] = pixel.0;
                        let pixel_brightness = r as u32 + g as u32 + b as u32;
                        let diff = avg_brightness.abs_diff(pixel_brightness);
                        variance_sum += diff * diff;
                    }
                }
            }

            let variance = variance_sum / pixel_count;
            variance < 2000 // Low variance indicates uniform lighting
        } else {
            false
        }
    }
}

/// Environment detector for tall grass, water, etc.
pub struct EnvironmentDetector {
    grass_threshold: u8,
    water_threshold: u8,
}

impl EnvironmentDetector {
    pub fn new() -> Self {
        Self {
            grass_threshold: 100,
            water_threshold: 120,
        }
    }
}

impl VisualDetector for EnvironmentDetector {
    fn detect(&self, context: &DetectionContext) -> DetectionResult<Vec<DetectionSignal>> {
        let start_time = std::time::Instant::now();
        let mut signals = Vec::new();

        if self.detect_tall_grass(&context.rgb) {
            signals.push(DetectionSignal {
                signal_type: DetectionSignalType::TallGrass,
                confidence: 0.7,
                location: None,
                metadata: DetectionMetadata::None,
            });
        }

        if self.detect_water(&context.rgb) {
            signals.push(DetectionSignal {
                signal_type: DetectionSignalType::Water,
                confidence: 0.8,
                location: None,
                metadata: DetectionMetadata::None,
            });
        }

        let signal_count = signals.len();
        let overall_confidence = signals.iter().map(|s| s.confidence).fold(0.0, f32::max);

        DetectionResult::new(
            signals,
            overall_confidence,
            format!("Environment detection found {} signals", signal_count),
        )
        .with_timing(start_time)
    }

    fn priority(&self) -> u8 {
        40 // Lower priority - environment is secondary to UI elements
    }

    fn name(&self) -> &'static str {
        "EnvironmentDetector"
    }

    fn can_process(&self, _context: &DetectionContext) -> bool {
        true
    }
}

impl EnvironmentDetector {
    fn detect_tall_grass(&self, rgb: &RgbImage) -> bool {
        let (width, height) = rgb.dimensions();
        let mut grass_pixels = 0;

        // Sample the bottom half of the image where grass typically appears
        for y in (height / 2..height).step_by(3) {
            for x in (0..width).step_by(3) {
                if let Some(pixel) = rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    // Green color detection for grass (safe arithmetic)
                    if g > self.grass_threshold
                        && g as u16 > r as u16 + 20
                        && g as u16 > b as u16 + 20
                    {
                        grass_pixels += 1;
                    }
                }
            }
        }

        grass_pixels > (width * height) / 2000 // Threshold for tall grass
    }

    fn detect_water(&self, rgb: &RgbImage) -> bool {
        let (width, height) = rgb.dimensions();
        let mut water_pixels = 0;

        for y in (0..height).step_by(3) {
            for x in (0..width).step_by(3) {
                if let Some(pixel) = rgb.get_pixel_checked(x, y) {
                    let [r, g, b] = pixel.0;
                    // Blue color detection for water (safe arithmetic)
                    if b > self.water_threshold
                        && b as u16 > r as u16 + 30
                        && b as u16 > g as u16 + 15
                    {
                        water_pixels += 1;
                    }
                }
            }
        }

        water_pixels > (width * height) / 1500 // Threshold for water areas
    }
}

impl Default for HPBarDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TextDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MenuDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LocationDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for EnvironmentDetector {
    fn default() -> Self {
        Self::new()
    }
}
