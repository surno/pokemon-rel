use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

use crate::{error::AppError, pipeline::EnrichedFrame};

pub struct ClientHandle {
    id: Uuid,
    tx: broadcast::Receiver<EnrichedFrame>,
}

impl ClientHandle {
    pub fn new(id: Uuid, tx: broadcast::Receiver<EnrichedFrame>) -> Self {
        Self { id, tx }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn get_frame(&mut self) -> Result<EnrichedFrame, AppError> {
        self.tx
            .try_recv()
            .map_err(|e| AppError::Client(format!("Error receiving frame: {}", e)))
    }
}

#[derive(Clone)]
pub struct ClientManager {
    sends: Arc<RwLock<HashMap<Uuid, broadcast::Sender<EnrichedFrame>>>>,
}

impl ClientManager {
    pub fn new() -> Self {
        Self {
            sends: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_client(&self, id: Uuid, tx: broadcast::Sender<EnrichedFrame>) {
        let mut senders = self.sends.blocking_write();
        senders.insert(id, tx);
    }

    pub fn remove_client(&self, id: &Uuid) {
        let mut senders = self.sends.blocking_write();
        senders.remove(id);
    }

    pub fn subscribe(&self, id: &Uuid) -> Option<ClientHandle> {
        let senders = self.sends.blocking_read();
        let sender = senders.get(id).cloned();
        if let Some(sender) = sender {
            let receiver = sender.subscribe();
            Some(ClientHandle::new(id.clone(), receiver))
        } else {
            None
        }
    }

    pub fn list_clients(&self) -> Vec<Uuid> {
        self.sends.blocking_read().keys().cloned().collect()
    }
}
