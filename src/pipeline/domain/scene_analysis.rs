use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneType {
    Battle,
    Menu,
    Overworld,
    Cutscene,
    Unknown,
}

pub struct SceneAnalysis {
    scene_type: SceneType,
    confidence: f32,
    timestamp: Instant,
}

impl SceneAnalysis {
    pub fn new(scene_type: SceneType, confidence: f32) -> Self {
        Self {
            scene_type,
            confidence,
            timestamp: Instant::now(),
        }
    }

    pub fn scene_type(&self) -> SceneType {
        self.scene_type
    }

    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }
}
