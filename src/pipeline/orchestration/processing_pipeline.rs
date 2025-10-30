use time::Duration;

pub struct ProcessingPipeline {
    enable_metrics: bool,
}

impl ProcessingPipeline {
    pub fn new() -> Self {
        Self {
            enable_metrics: false,
        }
    }
}

pub struct ProcessingPipelineBuilder {
    timeout: Option<Duration>,
    rate_limit: Option<(u64, Duration)>,
    enable_metrics: bool,
}

impl ProcessingPipelineBuilder {
    pub fn new() -> Self {
        Self {
            timeout: None,
            rate_limit: None,
            enable_metrics: false,
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn rate_limit(mut self, rate_limit: (u64, Duration)) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }

    pub fn enable_metrics(mut self, enable_metrics: bool) -> Self {
        self.enable_metrics = enable_metrics;
        self
    }
}

impl Default for ProcessingPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
