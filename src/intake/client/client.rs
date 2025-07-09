use crate::{
    error::AppError,
    intake::frame::{reader::FrameReader, visitor::FrameVisitor, writer::FrameWriter},
    pipeline::GameAction,
};
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

pub enum ClientCommand {
    SendAction(GameAction),
}

pub struct Client {
    id: Uuid,
    reader: Box<dyn FrameReader + Send + Sync>,
    writer: Box<dyn FrameWriter + Send + Sync>,
    visitor: Box<dyn FrameVisitor + Send + Sync>,
    action_channel: mpsc::Receiver<ClientCommand>,
}

impl Client {
    pub fn new(
        reader: Box<dyn FrameReader + Send + Sync>,
        writer: Box<dyn FrameWriter + Send + Sync>,
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
                            frame.accept(self.visitor.as_mut());
                        }
                        Err(e) => {
                            error!("Client pipeline for {:?} failed to read frame: {:?}", self.id, e);
                            return Err(AppError::Client(e.to_string()));
                        }
                    }
                }
                action = self.action_channel.recv() => {
                    match action {
                        Some(action) => match action {
                            ClientCommand::SendAction(_action) => {
                                info!("Client {:?} received action", self.id);
                            }
                        },
                        None => {
                            error!("Client {:?} action channel closed", self.id);
                        }
                    }
                }
            }
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}
