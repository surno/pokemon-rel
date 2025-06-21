use crate::error::AppError;
use crate::intake::frame::handler::FrameHandler;
use crate::pipeline::services::FanoutService;
use crate::pipeline::types::RawFrame;
use image::DynamicImage;
use tracing::debug;

pub struct PokemonFrameHandler {
    fanout_service: FanoutService,
}

impl PokemonFrameHandler {
    pub fn new(fanout_service: FanoutService) -> Self {
        Self { fanout_service }
    }
}

impl FrameHandler for PokemonFrameHandler {
    fn handle_ping(&self) -> Result<(), AppError> {
        debug!("Received ping");
        Ok(())
    }

    fn handle_handshake(&self, version: u32, name: String, program: u16) -> Result<(), AppError> {
        debug!(
            "Received handshake: version={}, name={}, program={}",
            version, name, program
        );
        Ok(())
    }

    fn handle_image(&self, image: DynamicImage) -> Result<(), AppError> {
        debug!(
            "Received image: width={}, height={}",
            image.width(),
            image.height()
        );
        let _raw_frame = RawFrame::new(
            image.width(),
            image.height(),
            image.as_rgb8().unwrap().to_vec(),
        );
        // TODO: do something with the action
        Ok(())
    }

    fn handle_image_gd2(&self, width: u32, height: u32, gd2_data: Vec<u8>) -> Result<(), AppError> {
        debug!("Received image GD2: width={}, height={}", width, height);
        let _ = RawFrame::new(width, height, gd2_data);
        Ok(())
    }

    fn handle_shutdown(&self) -> Result<(), AppError> {
        debug!("Received shutdown");
        Ok(())
    }
}
