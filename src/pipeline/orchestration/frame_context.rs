use crate::common::frame::Frame;
use std::sync::Arc;
use std::time::Instant;

pub struct Ingested {
    pub frame: Arc<Frame>,
    pub metrics: FrameMetrics,
    pub processing_start: Instant,
}

pub struct Analyzed {
    pub frame: Arc<Frame>,
    pub analysis: Analysis,
    pub metrics: FrameMetrics,
    pub processing_start: Instant,
}
pub enum FrameContext {
    Ingested(Ingested),
    Analyzed(Analyzed),
}

impl FrameContext {
    pub fn new(frame: Frame) -> Self {
        Self::Ingested(Ingested {
            frame: Arc::new(frame),
            metrics: FrameMetrics::new(),
            processing_start: Instant::now(),
        })
    }
}

/// Metrics collected during frame processing
#[derive(Debug, Clone, Default)]
pub struct FrameMetrics {
    pub scene_analysis_duration_us: u64,
    pub total_processing_duration_us: u64,
}

impl FrameMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_duration(&mut self, step: ProcessingStepType, duration_us: u64) {
        match step {
            ProcessingStepType::SceneAnalysis => self.scene_analysis_duration_us = duration_us,
            _ => {}
        }
    }

    pub fn finalize(&mut self, start_time: Instant) {
        self.total_processing_duration_us = start_time.elapsed().as_micros() as u64;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessingStepType {
    SceneAnalysis,
}
