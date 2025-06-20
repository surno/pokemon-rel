use crate::{intake::client::Client, pipeline::types::EnrichedFrame};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::sync::broadcast::Receiver;
use tracing::info;
use uuid::Uuid;

pub trait ClientManagerTrait: Send + Sync {
    fn get_frames_from_clients(&mut self) -> HashMap<Uuid, Option<EnrichedFrame>>;
    fn get_frame_from_client(&mut self, client_id: Uuid) -> Option<EnrichedFrame>;
    fn get_selected_client(&self) -> Option<Uuid>;
    fn set_selected_client(&self, client_id: Uuid);
    fn add_client(&mut self, client: Box<Client>);
    fn get_clients(&self) -> Vec<Uuid>;
    fn remove_client(&mut self, client_id: Uuid);
    fn subscribe_to_client(&mut self, client_id: Uuid, receiver: Receiver<EnrichedFrame>);
}

pub struct FrameReaderClientManager {
    pub clients: HashMap<Uuid, Box<Client>>,
    pub client_receiver: HashMap<Uuid, Receiver<EnrichedFrame>>,
    pub selected_client: RwLock<Option<Uuid>>,
}

impl FrameReaderClientManager {
    pub fn new() -> FrameReaderClientManager {
        Self {
            clients: HashMap::new(),
            client_receiver: HashMap::new(),
            selected_client: RwLock::new(None),
        }
    }
}

impl ClientManagerTrait for FrameReaderClientManager {
    fn get_frames_from_clients(&mut self) -> HashMap<Uuid, Option<EnrichedFrame>> {
        let mut frames = HashMap::new();
        for (client_id, receiver) in self.client_receiver.iter_mut() {
            if let Ok(frame) = receiver.try_recv() {
                frames.insert(*client_id, Some(frame));
            } else {
                frames.insert(*client_id, None);
            }
        }
        frames
    }

    fn get_frame_from_client(&mut self, client_id: Uuid) -> Option<EnrichedFrame> {
        if let Ok(frame) = self.client_receiver.get_mut(&client_id).unwrap().try_recv() {
            Some(frame)
        } else {
            None
        }
    }

    fn get_selected_client(&self) -> Option<Uuid> {
        self.selected_client.blocking_read().clone()
    }

    fn set_selected_client(&self, client_id: Uuid) {
        let _ = self.selected_client.blocking_write().insert(client_id);
    }

    fn add_client(&mut self, client: Box<Client>) {
        let client_id = client.id();
        info!("Adding client {}", client_id);
        self.clients.insert(client_id, client);
        let mut selected_client = self.selected_client.try_write().unwrap();
        if *selected_client == None {
            info!("No client selected, selecting {}", client_id);
            *selected_client = Some(client_id);
        }
    }

    fn subscribe_to_client(&mut self, client_id: Uuid, receiver: Receiver<EnrichedFrame>) {
        self.client_receiver.insert(client_id, receiver);
    }

    fn remove_client(&mut self, client_id: Uuid) {
        self.client_receiver.remove(&client_id);
        self.clients.remove(&client_id);
        let mut selected_client = self.selected_client.try_write().unwrap();
        if *selected_client == Some(client_id) {
            *selected_client = None;
        }
    }

    fn get_clients(&self) -> Vec<Uuid> {
        self.clients.keys().cloned().collect()
    }
}
