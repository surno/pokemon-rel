use crate::error::AppError;
use crate::pipeline::GameAction;
use crate::pipeline::services::{
    RLService,
    learning::{
        experience_collector::ExperienceCollector,
        reward::processor::reward_processor::RewardProcessor,
    },
    orchestration::{
        ProcessingStep,
        frame_context::{FrameContext, ProcessingStepType},
    },
};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::info;

/// Processing step that handles reward processing, experience collection, and policy updates
pub struct LearningStep {
    reward_processor: Arc<Mutex<dyn RewardProcessor>>,
    experience_collector: Arc<tokio::sync::Mutex<ExperienceCollector>>,
    rl_service: Arc<Mutex<RLService>>,
    policy_update_frequency: usize,
    actions_processed: usize,
}

impl LearningStep {
    pub fn new(
        reward_processor: Arc<Mutex<dyn RewardProcessor>>,
        experience_collector: Arc<tokio::sync::Mutex<ExperienceCollector>>,
        rl_service: Arc<Mutex<RLService>>,
    ) -> Self {
        Self {
            reward_processor,
            experience_collector,
            rl_service,
            policy_update_frequency: 50, // Save policy every 50 actions
            actions_processed: 0,
        }
    }

    pub fn with_policy_update_frequency(mut self, frequency: usize) -> Self {
        self.policy_update_frequency = frequency;
        self
    }

    fn game_action_to_index(action: &crate::pipeline::GameAction) -> usize {
        match action {
            crate::pipeline::GameAction::A => 0,
            crate::pipeline::GameAction::B => 1,
            crate::pipeline::GameAction::Up => 2,
            crate::pipeline::GameAction::Down => 3,
            crate::pipeline::GameAction::Left => 4,
            crate::pipeline::GameAction::Right => 5,
            crate::pipeline::GameAction::Start => 6,
            crate::pipeline::GameAction::Select => 7,
            crate::pipeline::GameAction::L => 8,
            crate::pipeline::GameAction::R => 9,
            crate::pipeline::GameAction::X => 10,
        }
    }
}

#[async_trait]
impl ProcessingStep for LearningStep {
    async fn process(&mut self, context: &mut FrameContext) -> Result<(), AppError> {
        // Only run learning step if image has changed
        if !context.image_changed {
            return Ok(());
        }

        let reward_start = Instant::now();
        let maybe_experience = {
            let mut reward_processor = self.reward_processor.lock().unwrap();
            let prediction = context
                .policy_prediction
                .as_ref()
                .cloned()
                .unwrap_or_default();
            reward_processor.process_frame(
                &context.frame,
                context.selected_action.unwrap_or(GameAction::A),
                prediction,
            )
        };
        info!("Reward processed in {:?}", reward_start.elapsed());
        let reward_duration = reward_start.elapsed().as_micros() as u64;
        context
            .metrics
            .record_duration(ProcessingStepType::RewardProcessing, reward_duration);

        if let Some(experience) = maybe_experience {
            let experience_start = Instant::now();
            {
                let mut collector = self.experience_collector.lock().await;
                collector.collect_experience(experience.clone()).await;
            }
            let experience_duration = experience_start.elapsed().as_micros() as u64;
            context.metrics.record_duration(
                ProcessingStepType::ExperienceCollection,
                experience_duration,
            );

            let selected_action = context.selected_action.unwrap_or(GameAction::A);

            // Online policy nudge using reward as advantage proxy
            let action_idx = Self::game_action_to_index(&selected_action);
            {
                let mut rl_service = self.rl_service.lock().unwrap();
                rl_service.nudge_action(action_idx, experience.reward);
            }

            // Periodically persist the policy
            self.actions_processed += 1;
            if self.actions_processed % self.policy_update_frequency == 0 {
                let rl_service = self.rl_service.lock().unwrap();
                rl_service.save_now_blocking();
                tracing::info!("Policy saved after {} actions", self.actions_processed);
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "LearningStep"
    }
}
