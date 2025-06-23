use crate::{
    error::AppError,
    intake::frame::{reader::FrameReader, visitor::FrameVisitor},
};
use tracing::{error, info};
use uuid::Uuid;

pub struct Client {
    id: Uuid,
    reader: Box<dyn FrameReader + Send + Sync>,
    visitor: Box<dyn FrameVisitor + Send + Sync>,
}

impl Client {
    pub fn new(
        reader: Box<dyn FrameReader + Send + Sync>,
        visitor: Box<dyn FrameVisitor + Send + Sync>,
    ) -> Box<Client> {
        let id = Uuid::new_v4();
        Box::new(Client {
            id,
            reader,
            visitor,
        })
    }

    pub async fn start(&mut self) -> Result<(), AppError> {
        info!("Running client pipeline for {:?}", self.id);
        loop {
            let next_message = self.reader.read().await;
            match next_message {
                Ok(frame) => {
                    info!("Client {:?} received frame", self.id);
                    frame.accept(self.visitor.as_mut());
                }
                Err(e) => {
                    error!("Client pipeline for {:?} handled message: {:?}", self.id, e);
                    return Err(AppError::Client(e.to_string()));
                }
            }
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}
