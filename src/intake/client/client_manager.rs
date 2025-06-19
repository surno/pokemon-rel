use crate::pipeline::types::SharedFrame;
use std::collections::HashMap;
use tokio::sync::broadcast::Receiver;
use tracing::info;
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

    pub fn get_frames_from_clients(&mut self) -> HashMap<Uuid, Option<SharedFrame>> {
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

    pub fn add_client(&mut self, client_id: Uuid, receiver: Receiver<SharedFrame>) {
        info!("Adding client {}", client_id);
        self.client_receiver.insert(client_id, receiver);
        if self.selected_client.is_none() {
            info!("No client selected, selecting {}", client_id);
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
