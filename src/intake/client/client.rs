use crate::{
    error::AppError,
    intake::frame::{reader::FrameReader, visitor::FrameVisitor, writer::FramedWriter},
    pipeline::GameAction,
};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::supervisor::ClientCommand;

pub struct Client {
    id: Uuid,
    reader: Box<dyn FrameReader + Send + Sync>,
    writer: Box<dyn FramedWriter + Send + Sync>,
    visitor: Box<dyn FrameVisitor + Send + Sync>,
    action_channel: mpsc::Receiver<ClientCommand>,
}

impl Client {
    pub fn new(
        reader: Box<dyn FrameReader + Send + Sync>,
        writer: Box<dyn FramedWriter + Send + Sync>,
        visitor: Box<dyn FrameVisitor + Send + Sync>,
        action_channel: mpsc::Receiver<ClientCommand>,
    ) -> Client {
        let id = Uuid::new_v4();
        Client {
            id,
            reader,
            writer,
            visitor,
            action_channel,
        }
    }

    pub async fn start(&mut self) -> Result<(), AppError> {
        info!("Running client pipeline for {:?}", self.id);
        loop {
            tokio::select! {
                next_message = self.reader.read() => {
                    match next_message {
                        Ok(frame) => {
                            if let Err(e) = frame.accept(self.visitor.as_mut()) {
                                // Log the error but don't crash the client
                                tracing::warn!("Frame processing error for client {:?}: {:?}", self.id, e);
                            }
                        }
                        Err(e) => {
                            // This is an expected error when the connection closes.
                            tracing::debug!("Client reader for {:?} disconnected: {:?}. Shutting down client.", self.id, e);
                            break;
                        }
                    }
                }
                action = self.action_channel.recv() => {
                    match action {
                        Some(action) => match action {
                            ClientCommand::SendAction(action) => {
                                info!("Client {:?} sending action {:?}", self.id, action);
                                if let Err(e) = self.writer.send_action(action).await {
                                    error!("Client {:?} failed to send action: {:?}", self.id, e);
                                }
                            }
                        },
                        None => {
                            info!("Client {:?} action channel closed. Shutting down.", self.id);
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}
