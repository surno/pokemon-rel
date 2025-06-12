use crate::pipeline::types::SharedFrame;
use std::collections::HashMap;
use tokio::sync::broadcast::Receiver;
use uuid::Uuid;

#[derive(Debug)]
pub struct ClientManager {
    pub client_receiver: HashMap<Uuid, Receiver<SharedFrame>>,
    pub client_frames: HashMap<Uuid, SharedFrame>,
    pub selected_client: Option<Uuid>,
}

impl ClientManager {
    pub fn new() -> Self {
        Self {
            client_receiver: HashMap::new(),
            client_frames: HashMap::new(),
            selected_client: None,
        }
    }

    pub fn add_client(&mut self, client_id: Uuid, receiver: Receiver<SharedFrame>) {
        self.client_receiver.insert(client_id, receiver);
        if self.selected_client.is_none() {
            self.selected_client = Some(client_id);
        }
    }

    pub fn remove_client(&mut self, client_id: Uuid) {
        self.client_receiver.remove(&client_id);
        self.client_frames.remove(&client_id);
        if self.selected_client == Some(client_id) {
            self.selected_client = None;
        }
    }
}
