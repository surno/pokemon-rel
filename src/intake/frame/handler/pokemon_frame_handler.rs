use crate::error::AppError;
use crate::intake::frame::handler::FrameHandler;
use crate::pipeline::services::FanoutService;
use crate::pipeline::types::RawFrame;
use std::future::Future;
use std::pin::Pin;
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
    fn handle_ping(&self) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'static>> {
        debug!("Received ping");
        Box::pin(async { Ok(()) })
    }

    fn handle_handshake(
        &self,
        version: u32,
        name: String,
        program: u16,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'static>> {
        debug!(
            "Received handshake: version={}, name={}, program={}",
            version, name, program
        );
        Box::pin(async { Ok(()) })
    }

    fn handle_image(
        &self,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'static>> {
        debug!("Received image: width={}, height={}", width, height);
        let _raw_frame = RawFrame::new(width, height, pixels);
        Box::pin(async move {
            // TODO: do something with the action
            Ok(())
        })
    }

    fn handle_image_gd2(
        &self,
        width: u32,
        height: u32,
        gd2_data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'static>> {
        debug!("Received image GD2: width={}, height={}", width, height);
        let _ = RawFrame::new(width, height, gd2_data);
        Box::pin(async move { Ok(()) })
    }

    fn handle_shutdown(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'static>> {
        debug!("Received shutdown");
        Box::pin(async { Ok(()) })
    }
}
