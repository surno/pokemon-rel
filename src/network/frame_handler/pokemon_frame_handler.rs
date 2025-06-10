use crate::error::FrameHandlerError;
use crate::network::frame::Frame;
use crate::pipeline::services::{ActionService, PreprocessingService, RLService};
use crate::pipeline::types::{EnrichedFrame, GameAction, RLPrediction};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

#[derive(Debug, Clone)]
pub struct PokemonFrameHandler {}

impl FrameHandler for PokemonFrameHandler {
    fn handle_ping(&self) -> Result<(), FrameError> {
        debug!("Received ping");
        Ok(())
    }

    fn handle_handshake(&self, version: u32, name: String, program: u16) -> Result<(), FrameError> {
        debug!(
            "Received handshake: version={}, name={}, program={}",
            version, name, program
        );
        Ok(())
    }

    fn handle_image(&self, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), FrameError> {
        debug!("Received image: width={}, height={}", width, height);
        Ok(())
    }

    fn handle_shutdown(&self) -> Result<(), FrameError> {
        debug!("Received shutdown");
        Ok(())
    }
}
