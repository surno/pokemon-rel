use crate::error::AppError;
use crate::intake::frame::frame_handler::FrameHandler;
use crate::pipeline::services::{FanoutService, fanout_service};
use crate::pipeline::types::RawFrame;
use std::future::Future;
use std::pin::Pin;
use tower::Service;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct PokemonFrameHandler {
    fanout_service: FanoutService,
}

impl PokemonFrameHandler {
    pub fn new(fanout_service: FanoutService) -> Self {
        Self { fanout_service }
    }
}

impl FrameHandler for PokemonFrameHandler {
    fn handle_ping<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        debug!("Received ping");
        Box::pin(async { Ok(()) })
    }

    fn handle_handshake<'a>(
        &'a self,
        version: u32,
        name: String,
        program: u16,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        debug!(
            "Received handshake: version={}, name={}, program={}",
            version, name, program
        );
        Box::pin(async { Ok(()) })
    }

    fn handle_image<'a>(
        &'a self,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        debug!("Received image: width={}, height={}", width, height);
        let raw_frame = RawFrame::new(width, height, pixels);
        let mut fanout_service = self.fanout_service.clone();
        Box::pin(async move {
            fanout_service.call(raw_frame).await?;
            // TODO: do something with the action
            Ok(())
        })
    }

    fn handle_image_gd2<'a>(
        &'a self,
        width: u32,
        height: u32,
        gd2_data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        debug!("Received image GD2: width={}, height={}", width, height);
        let raw_frame = RawFrame::new(width, height, gd2_data);
        let mut fanout_service = self.fanout_service.clone();
        Box::pin(async move {
            fanout_service.call(raw_frame).await?;
            // TODO: do something with the action
            Ok(())
        })
    }

    fn handle_shutdown<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
        debug!("Received shutdown");
        Box::pin(async { Ok(()) })
    }
}
