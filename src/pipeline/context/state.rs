use crate::pipeline::domain::scene_analysis::SceneAnalysis;

// Markers to track the state of the frame processing pipeline
pub struct IngestedState;
pub struct AnalyzedState {
    pub(super) analysis: SceneAnalysis,
}
// Optional: Add trait for introspection
pub trait ProcessingState: 'static {
    fn state_name() -> &'static str;
}

impl ProcessingState for IngestedState {
    fn state_name() -> &'static str {
        "Ingested"
    }
}

impl ProcessingState for AnalyzedState {
    fn state_name() -> &'static str {
        "Analyzed"
    }
}
