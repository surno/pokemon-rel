use crate::pipeline::services::learning::experience_collector::Experience;
use crate::pipeline::types::EnrichedFrame;

pub trait RewardProcessor {
    fn process_frame(&mut self, frame: &EnrichedFrame) -> Option<Experience>;
}
