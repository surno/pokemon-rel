use std::sync::Arc;

use crate::{error::AppError, intake::frame::reader::FrameReader, pipeline::EnrichedFrame};
use tokio::sync::broadcast;
use tracing::{error, info};
use uuid::Uuid;

pub struct Client {
    id: Uuid,
    reader: Box<dyn FrameReader + Send + Sync>,
    subscription: Arc<broadcast::Sender<Arc<EnrichedFrame>>>,
}

impl Client {
    pub fn new(reader: Box<dyn FrameReader + Send + Sync>) -> Box<Client> {
        let id = Uuid::new_v4();
        let (tx, _) = broadcast::channel::<Arc<EnrichedFrame>>(60);
        let tx = Arc::new(tx);
        Box::new(Client {
            id,
            reader,
            subscription: tx.clone(),
        })
    }

    pub async fn start(&mut self) -> Result<(), AppError> {
        info!("Running client pipeline for {:?}", self.id);
        loop {
            let next_message = self.reader.read().await;
            match next_message {
                Ok(_) => {
                    info!("Client {:?} received frame", self.id);
                    // self.subscriptions.send(Arc::new(frame));
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
