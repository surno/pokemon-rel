use crate::pipeline::services::{
    action_service::ActionService, preprocessing::PreprocessingService, rl_service::RLService,
};
use crate::pipeline::types::{GameAction, RawFrame};
use tower::ServiceExt;
use tower::util::BoxService;
use tower::{Service, ServiceBuilder};

#[derive(Debug)]
pub struct Pipeline {
    service: BoxService<RawFrame, GameAction, crate::error::PipelineError>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        let service = ServiceBuilder::new()
            //TODO: Add middleware here
            .service(PreprocessingService)
            .and_then(|enriched_frame| async move {
                let rl_prediction = RLService.call(enriched_frame).await?;
                Ok::<_, crate::error::PipelineError>(rl_prediction)
            })
            .and_then(|rl_prediction| async move {
                let game_action = ActionService.call(rl_prediction).await?;
                Ok::<_, crate::error::PipelineError>(game_action)
            })
            .boxed();

        Self { service }
    }

    pub async fn process_frame(
        &mut self,
        frame: RawFrame,
    ) -> Result<GameAction, crate::error::PipelineError> {
        let response = self.service.call(frame).await?;
        Ok(response)
    }
}
