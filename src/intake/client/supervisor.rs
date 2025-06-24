use crate::{error::AppError, intake::client::client::ClientCommand, pipeline::GameAction};
use std::collections::HashMap;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use uuid::Uuid;

pub struct ClientEntry {
    pub id: Uuid,
    pub client_task: JoinHandle<Result<(), AppError>>,
    pub action_channel: mpsc::Sender<ClientCommand>,
}

pub enum ClientSupervisorCommand {
    AddClient {
        entry: ClientEntry,
        responder: oneshot::Sender<Uuid>,
    },
    RemoveClient {
        id: Uuid,
        responder: oneshot::Sender<()>,
    },
    ListClients {
        responder: oneshot::Sender<Vec<Uuid>>,
    },
    SendAction {
        id: Uuid,
        action: GameAction,
    },
}

pub struct ClientSupervisor {
    clients: HashMap<Uuid, ClientEntry>,
}

impl ClientSupervisor {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn add_client(&mut self, client_entry: ClientEntry) {
        self.clients.insert(client_entry.id, client_entry);
    }

    pub fn remove_client(&mut self, client_id: Uuid) {
        if let Some(entry) = self.clients.remove(&client_id) {
            entry.client_task.abort();
        }
    }

    pub fn list_clients(&self) -> Vec<Uuid> {
        self.clients.keys().cloned().collect()
    }

    pub fn handle_command(&mut self, command: ClientSupervisorCommand) {
        match command {
            ClientSupervisorCommand::AddClient { entry, responder } => {
                let id = entry.id;
                self.add_client(entry);
                let _ = responder.send(id);
            }
            ClientSupervisorCommand::RemoveClient { id, responder } => {
                self.remove_client(id);
                let _ = responder.send(());
            }
            ClientSupervisorCommand::ListClients { responder } => {
                let _ = responder.send(self.list_clients());
            }
            ClientSupervisorCommand::SendAction { id, action } => {
                if let Some(entry) = self.clients.get(&id) {
                    let _ = entry.action_channel.send(ClientCommand::SendAction(action));
                }
            }
        }
    }
}
