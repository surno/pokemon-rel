/// Configuration for scene analysis with tunable parameters
#[derive(Debug, Clone)]
pub struct SceneAnalysisConfig {
    pub detection_sensitivity: f32,
    pub confidence_threshold: f32,
    pub enabled_detectors: Vec<DetectorType>,
    pub region_sampling: RegionSamplingConfig,
    pub color_thresholds: ColorThresholds,
    pub performance_mode: PerformanceMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetectorType {
    HPBar,
    BattleMenu,
    MainMenu,
    DialogBox,
    TextBlock,
    MenuCursor,
    TallGrass,
    Water,
    Indoor,
    PokemonCenter,
    Gym,
    Cave,
    City,
    Town,
    Route,
    Building,
    Shiny,
    Pokemon,
    BagMenu,
}

#[derive(Debug, Clone)]
pub struct RegionSamplingConfig {
    pub sample_step: u32,
    pub min_region_size: u32,
    pub max_regions_per_frame: usize,
    pub enable_adaptive_sampling: bool,
}

#[derive(Debug, Clone)]
pub struct ColorThresholds {
    pub hp_bar_green_threshold: u8,
    pub hp_bar_red_threshold: u8,
    pub text_contrast_threshold: u8,
    pub menu_border_threshold: u8,
    pub grass_green_min: u8,
    pub water_blue_min: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceMode {
    Speed,    // Minimal detection for maximum FPS
    Balanced, // Good balance of speed and accuracy
    Accuracy, // Maximum accuracy, slower processing
}

impl Default for SceneAnalysisConfig {
    fn default() -> Self {
        Self {
            detection_sensitivity: 0.7,
            confidence_threshold: 0.6,
            enabled_detectors: vec![
                DetectorType::HPBar,
                DetectorType::BattleMenu,
                DetectorType::MainMenu,
                DetectorType::DialogBox,
                DetectorType::TextBlock,
            ],
            region_sampling: RegionSamplingConfig::default(),
            color_thresholds: ColorThresholds::default(),
            performance_mode: PerformanceMode::Balanced,
        }
    }
}

impl Default for RegionSamplingConfig {
    fn default() -> Self {
        Self {
            sample_step: 4,
            min_region_size: 16,
            max_regions_per_frame: 50,
            enable_adaptive_sampling: true,
        }
    }
}

impl Default for ColorThresholds {
    fn default() -> Self {
        Self {
            hp_bar_green_threshold: 120, // Lowered for DS color palette
            hp_bar_red_threshold: 130,   // Adjusted for DS red
            text_contrast_threshold: 70, // Lowered for DS text
            menu_border_threshold: 60,   // DS menu borders are softer
            grass_green_min: 60,         // DS grass is less saturated
            water_blue_min: 80,          // DS water is darker
        }
    }
}

impl SceneAnalysisConfig {
    /// Configuration optimized for speed (minimal detection)
    pub fn speed_optimized() -> Self {
        Self {
            detection_sensitivity: 0.5,
            confidence_threshold: 0.5,
            enabled_detectors: vec![
                DetectorType::MainMenu,
                DetectorType::DialogBox,
                DetectorType::BattleMenu,
            ],
            region_sampling: RegionSamplingConfig {
                sample_step: 8,
                min_region_size: 32,
                max_regions_per_frame: 20,
                enable_adaptive_sampling: false,
            },
            color_thresholds: ColorThresholds::default(),
            performance_mode: PerformanceMode::Speed,
        }
    }

    /// Configuration optimized for accuracy (comprehensive detection)
    pub fn accuracy_optimized() -> Self {
        Self {
            detection_sensitivity: 0.9,
            confidence_threshold: 0.8,
            enabled_detectors: vec![
                DetectorType::HPBar,
                DetectorType::BattleMenu,
                DetectorType::MainMenu,
                DetectorType::DialogBox,
                DetectorType::TextBlock,
                DetectorType::MenuCursor,
                DetectorType::TallGrass,
                DetectorType::Water,
                DetectorType::Indoor,
                DetectorType::PokemonCenter,
                DetectorType::Gym,
                DetectorType::Cave,
                DetectorType::City,
                DetectorType::Town,
                DetectorType::Route,
                DetectorType::Building,
                DetectorType::Shiny,
                DetectorType::Pokemon,
                DetectorType::BagMenu,
            ],
            region_sampling: RegionSamplingConfig {
                sample_step: 2,
                min_region_size: 8,
                max_regions_per_frame: 100,
                enable_adaptive_sampling: true,
            },
            color_thresholds: ColorThresholds::default(),
            performance_mode: PerformanceMode::Accuracy,
        }
    }

    /// Configuration for Pokemon-specific detection
    pub fn pokemon_optimized() -> Self {
        Self {
            detection_sensitivity: 0.8,
            confidence_threshold: 0.7,
            enabled_detectors: vec![
                DetectorType::HPBar,
                DetectorType::BattleMenu,
                DetectorType::MainMenu,
                DetectorType::DialogBox,
                DetectorType::TallGrass,
                DetectorType::PokemonCenter,
                DetectorType::Gym,
                DetectorType::Shiny,
                DetectorType::Pokemon,
                DetectorType::BagMenu,
                DetectorType::MenuCursor,
            ],
            region_sampling: RegionSamplingConfig::default(),
            color_thresholds: ColorThresholds {
                hp_bar_green_threshold: 140,
                hp_bar_red_threshold: 140,
                text_contrast_threshold: 90,
                menu_border_threshold: 70,
                grass_green_min: 80,
                water_blue_min: 100,
            },
            performance_mode: PerformanceMode::Balanced,
        }
    }

    /// Validate configuration parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.detection_sensitivity < 0.0 || self.detection_sensitivity > 1.0 {
            return Err("Detection sensitivity must be between 0.0 and 1.0".to_string());
        }

        if self.confidence_threshold < 0.0 || self.confidence_threshold > 1.0 {
            return Err("Confidence threshold must be between 0.0 and 1.0".to_string());
        }

        if self.enabled_detectors.is_empty() {
            return Err("At least one detector must be enabled".to_string());
        }

        if self.region_sampling.sample_step == 0 {
            return Err("Sample step must be greater than 0".to_string());
        }

        Ok(())
    }

    /// Enable a specific detector type
    pub fn enable_detector(mut self, detector_type: DetectorType) -> Self {
        if !self.enabled_detectors.contains(&detector_type) {
            self.enabled_detectors.push(detector_type);
        }
        self
    }

    /// Disable a specific detector type
    pub fn disable_detector(mut self, detector_type: DetectorType) -> Self {
        self.enabled_detectors.retain(|d| *d != detector_type);
        self
    }

    /// Set performance mode
    pub fn with_performance_mode(mut self, mode: PerformanceMode) -> Self {
        self.performance_mode = mode;
        self
    }

    /// Set detection sensitivity
    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.detection_sensitivity = sensitivity.clamp(0.0, 1.0);
        self
    }
}
