use crate::pipeline::{Scene, State};
use image::{DynamicImage, RgbImage};
use std::sync::Arc;
use std::time::Instant;

/// Core detection context that flows through the detection pipeline
#[derive(Clone)]
pub struct DetectionContext {
    pub image: Arc<DynamicImage>,
    pub rgb: Arc<RgbImage>,
    pub dimensions: (u32, u32),
    pub region: Option<ImageRegion>,
    pub previous_signals: Vec<DetectionSignal>,
    pub processing_start: Instant,
}

impl DetectionContext {
    pub fn new(image: DynamicImage) -> Self {
        let rgb = Arc::new(image.to_rgb8());
        let dimensions = rgb.dimensions();

        Self {
            image: Arc::new(image),
            rgb,
            dimensions,
            region: None,
            previous_signals: Vec::new(),
            processing_start: Instant::now(),
        }
    }

    pub fn with_region(mut self, region: ImageRegion) -> Self {
        self.region = Some(region);
        self
    }

    pub fn add_signal(&mut self, signal: DetectionSignal) {
        self.previous_signals.push(signal);
    }

    pub fn has_signal(&self, signal_type: DetectionSignalType) -> bool {
        self.previous_signals
            .iter()
            .any(|s| s.signal_type == signal_type)
    }

    pub fn get_signal_confidence(&self, signal_type: DetectionSignalType) -> Option<f32> {
        self.previous_signals
            .iter()
            .find(|s| s.signal_type == signal_type)
            .map(|s| s.confidence)
    }
}

/// Result of a detection operation with confidence and reasoning
#[derive(Debug, Clone)]
pub struct DetectionResult<T> {
    pub result: T,
    pub confidence: f32,
    pub reasoning: String,
    pub processing_time_us: u64,
    pub signals_detected: Vec<DetectionSignal>,
}

impl<T> DetectionResult<T> {
    pub fn new(result: T, confidence: f32, reasoning: String) -> Self {
        Self {
            result,
            confidence,
            reasoning,
            processing_time_us: 0,
            signals_detected: Vec::new(),
        }
    }

    pub fn with_signals(mut self, signals: Vec<DetectionSignal>) -> Self {
        self.signals_detected = signals;
        self
    }

    pub fn with_timing(mut self, start_time: Instant) -> Self {
        self.processing_time_us = start_time.elapsed().as_micros() as u64;
        self
    }
}

/// Individual detection signals that can be combined for scene recognition
#[derive(Debug, Clone, PartialEq)]
pub struct DetectionSignal {
    pub signal_type: DetectionSignalType,
    pub confidence: f32,
    pub location: Option<ImageRegion>,
    pub metadata: DetectionMetadata,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetectionSignalType {
    // UI Elements
    HPBar,
    BattleMenu,
    MainMenu,
    DialogBox,
    TextBlock,
    MenuCursor,

    // Environment
    TallGrass,
    Water,
    Indoor,
    Outdoor,

    // Locations
    PokemonCenter,
    Gym,
    Cave,
    City,
    Town,
    Route,
    Building,

    // Game State
    BattleTurn,
    MenuOption,
    PlayerPosition,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetectionMetadata {
    None,
    Position(u32, u32),
    Count(usize),
    Color(u8, u8, u8),
    Text(String),
    Numeric(f32),
}

/// Rectangular region of an image for focused analysis
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl ImageRegion {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn full_image(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }

    pub fn top_quarter(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height / 4)
    }

    pub fn bottom_quarter(width: u32, height: u32) -> Self {
        Self::new(0, (height * 3) / 4, width, height / 4)
    }

    pub fn center_half(width: u32, height: u32) -> Self {
        Self::new(width / 4, height / 4, width / 2, height / 2)
    }

    pub fn contains_point(&self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    pub fn area(&self) -> u32 {
        self.width * self.height
    }
}

/// Strategy pattern for scene detection
pub trait SceneDetector: Send + Sync {
    fn detect_scene(&self, context: &DetectionContext) -> DetectionResult<Scene>;
    fn name(&self) -> &'static str;
    fn supported_scenes(&self) -> Vec<Scene>;
}

/// Strategy pattern for game state analysis
pub trait GameStateAnalyzer: Send + Sync {
    fn analyze_state(&self, context: &DetectionContext, scene: Scene) -> DetectionResult<State>;
    fn name(&self) -> &'static str;
}

/// Visual detector for specific elements (Chain of Responsibility)
pub trait VisualDetector: Send + Sync {
    fn detect(&self, context: &DetectionContext) -> DetectionResult<Vec<DetectionSignal>>;
    fn priority(&self) -> u8; // Higher priority = processed first
    fn name(&self) -> &'static str;
    fn can_process(&self, context: &DetectionContext) -> bool;
}
