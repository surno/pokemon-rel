/// Detection pipeline using Chain of Responsibility pattern
use super::core::{DetectionContext, DetectionResult, VisualDetector};
use std::time::Instant;

/// Pipeline that processes visual detectors in priority order
pub struct DetectionPipeline {
    detectors: Vec<Box<dyn VisualDetector>>,
    enable_early_termination: bool,
    max_processing_time_us: u64,
}

impl DetectionPipeline {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
            enable_early_termination: true,
            max_processing_time_us: 10_000, // 10ms max processing time
        }
    }

    pub fn add_detector(mut self, detector: Box<dyn VisualDetector>) -> Self {
        self.detectors.push(detector);
        // Sort by priority (higher first)
        self.detectors
            .sort_by(|a, b| b.priority().cmp(&a.priority()));
        self
    }

    pub fn with_early_termination(mut self, enabled: bool) -> Self {
        self.enable_early_termination = enabled;
        self
    }

    pub fn with_max_processing_time(mut self, max_time_us: u64) -> Self {
        self.max_processing_time_us = max_time_us;
        self
    }

    /// Process all detectors and collect signals
    pub fn process(&mut self, mut context: DetectionContext) -> DetectionResult<DetectionContext> {
        let start_time = Instant::now();
        let mut all_signals = Vec::new();
        let mut processing_log = Vec::new();

        for detector in &mut self.detectors {
            // Check if we should process this detector
            if !detector.can_process(&context) {
                continue;
            }

            // Check processing time limit
            if start_time.elapsed().as_micros() as u64 > self.max_processing_time_us {
                processing_log.push(format!("Stopped processing due to time limit"));
                break;
            }

            // Run the detector
            let detector_start = Instant::now();
            let result = detector.detect(&context);
            let detector_time = detector_start.elapsed().as_micros() as u64;

            processing_log.push(format!(
                "{}: {} signals in {}us",
                detector.name(),
                result.result.len(),
                detector_time
            ));

            // Add signals to context and collection
            for signal in result.result {
                context.add_signal(signal.clone());
                all_signals.push(signal);
            }

            // Early termination if high confidence signal found
            if self.enable_early_termination && result.confidence > 0.9 {
                processing_log.push(format!("Early termination due to high confidence"));
                break;
            }
        }

        let _total_time = start_time.elapsed().as_micros() as u64;
        let overall_confidence = all_signals.iter().map(|s| s.confidence).fold(0.0, f32::max);

        DetectionResult::new(
            context,
            overall_confidence,
            format!(
                "Pipeline processed {} detectors, found {} signals: {}",
                processing_log.len(),
                all_signals.len(),
                processing_log.join("; ")
            ),
        )
        .with_timing(start_time)
    }

    /// Get statistics about the pipeline
    pub fn get_stats(&self) -> PipelineStats {
        PipelineStats {
            total_detectors: self.detectors.len(),
            detector_names: self
                .detectors
                .iter()
                .map(|d| d.name().to_string())
                .collect(),
            early_termination_enabled: self.enable_early_termination,
            max_processing_time_us: self.max_processing_time_us,
        }
    }

    /// Get detector priorities for debugging
    pub fn get_detector_priorities(&self) -> Vec<(String, u8)> {
        self.detectors
            .iter()
            .map(|d| (d.name().to_string(), d.priority()))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub total_detectors: usize,
    pub detector_names: Vec<String>,
    pub early_termination_enabled: bool,
    pub max_processing_time_us: u64,
}

impl Default for DetectionPipeline {
    fn default() -> Self {
        Self::new()
    }
}
