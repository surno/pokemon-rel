use std::time::Duration;

/// Metrics collected during frame processing
pub struct FrameMetrics {
    analysis_duration: Option<Duration>,
}

impl FrameMetrics {
    pub fn new() -> Self {
        Self {
            analysis_duration: None,
        }
    }

    pub fn record_analysis_duration(&mut self, duration: Duration) {
        self.analysis_duration = Some(duration);
    }
}
