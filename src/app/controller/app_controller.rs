use std::sync::{Arc, RwLock};

use tokio::sync::mpsc::{self, Receiver, Sender};
use tower::Service;

use crate::error::AppError;
use crate::intake::client::manager::ClientManagerHandle;
use crate::intake::client::supervisor::ClientSupervisorCommand;
use crate::pipeline::ActionService;
use crate::pipeline::services::learning::experience_collector::ExperienceCollector;
use crate::pipeline::services::learning::reward::calculator::navigation_reward::NavigationRewardCalculator;
use crate::pipeline::services::learning::reward::processor::multi_objective_reward_processor::MultiObjectiveRewardProcessor;
use crate::pipeline::types::{EnrichedFrame, GameAction};

pub struct AppController {
    reward_processor: MultiObjectiveRewardProcessor,
    experience_collector: Arc<RwLock<ExperienceCollector>>,
    action_service: ActionService,
    result_tx: Sender<EnrichedFrame>,
    frame_rx: Receiver<EnrichedFrame>,
    action_tx: mpsc::Sender<ClientSupervisorCommand>,
}

impl AppController {
    pub fn new(
        result_tx: Sender<EnrichedFrame>,
        frame_rx: Receiver<EnrichedFrame>,
        action_tx: mpsc::Sender<ClientSupervisorCommand>,
    ) -> Self {
        let (training_tx, _training_rx) = mpsc::channel(1000);
        Self {
            reward_processor: MultiObjectiveRewardProcessor::new(Box::new(
                NavigationRewardCalculator::default(),
            )),
            experience_collector: Arc::new(RwLock::new(ExperienceCollector::new(
                1000,
                training_tx,
            ))),
            action_service: ActionService,
            result_tx,
            frame_rx,
            action_tx,
        }
    }

    pub async fn run(&mut self) -> Result<(), AppError> {
        loop {
            if let Some(frame) = self.frame_rx.recv().await {
                let id = frame.id;
                // Annotate the frame with data
                // Scene annotation now handled by the new pipeline architecture
                let enriched_frame = frame; // Pass through for now

                // get prediction
                // get action
                let action = self.action_service.call(enriched_frame.clone()).await?;

                // send action to the agent
                self.action_tx
                    .send(ClientSupervisorCommand::SendAction { id, action })
                    .await
                    .map_err(|e| AppError::Client(e.to_string()))?;

                // process rewards

                // send to ui.
                self.result_tx
                    .send(enriched_frame)
                    .await
                    .map_err(|e| AppError::Client(e.to_string()))?;
            }
        }
    }
}
