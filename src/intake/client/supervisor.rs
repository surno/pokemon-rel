use crate::pipeline::GameAction;
use tokio::sync::{mpsc, oneshot};
use tracing::error;
use uuid::Uuid;

pub struct ClientEntry {
    pub id: Uuid,
    pub client_task: tokio::task::JoinHandle<Result<(), crate::error::AppError>>,
    pub action_channel: mpsc::Sender<ClientCommand>,
}

pub enum ClientCommand {
    SendAction(GameAction),
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
    clients: Vec<ClientEntry>,
}

impl ClientSupervisor {
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
        }
    }

    pub fn add_client(&mut self, client_entry: ClientEntry) {
        self.clients.push(client_entry);
    }

    pub fn remove_client(&mut self, client_id: Uuid) {
        self.clients.retain(|entry| entry.id != client_id);
    }

    pub fn list_clients(&self) -> Vec<Uuid> {
        self.clients.iter().map(|entry| entry.id).collect()
    }

    pub fn handle_command(&mut self, command: ClientSupervisorCommand) {
        match command {
            ClientSupervisorCommand::AddClient { entry, responder } => {
                let id = entry.id;
                self.clients.push(entry);
                let _ = responder.send(id);
            }
            ClientSupervisorCommand::RemoveClient { id, responder } => {
                self.clients.retain(|entry| entry.id != id);
                let _ = responder.send(());
            }
            ClientSupervisorCommand::ListClients { responder } => {
                let ids = self.clients.iter().map(|entry| entry.id).collect();
                let _ = responder.send(ids);
            }
            ClientSupervisorCommand::SendAction { id, action } => {
                if let Some(entry) = self.clients.iter().find(|entry| entry.id == id) {
                    if let Err(e) = entry
                        .action_channel
                        .try_send(ClientCommand::SendAction(action))
                    {
                        error!("Failed to send action to client {}: {}", id, e);
                    }
                }
            }
        }
    }
}
