use crate::{
    error::AppError,
    intake::{
        client::{Client, client::ClientCommand},
        frame::{reader::FrameReader, visitor::FrameDelegatingVisitor, writer::FrameWriter},
    },
    pipeline::EnrichedFrame,
};

use std::collections::HashMap;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use uuid::Uuid;

struct ClientEntry {
    id: Uuid,
    client_task: JoinHandle<Result<(), AppError>>,
    action_channel: mpsc::Sender<ClientCommand>,
}

enum ClientSupervisorCommand {
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
}

struct ClientSupervisor {
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
        }
    }
}

pub struct ClientManager {
    frame_tx: mpsc::Sender<EnrichedFrame>,
    command_tx: mpsc::Sender<ClientSupervisorCommand>,
    frame_handler: JoinHandle<()>,
}

impl ClientManager {
    pub fn new() -> Self {
        // Generate a channel for receiving frames from the client
        let (frame_tx, mut frame_rx) = mpsc::channel::<EnrichedFrame>(100);
        let (command_tx, mut command_rx) = mpsc::channel::<ClientSupervisorCommand>(100);
        let frame_handler = tokio::spawn(async move {
            let mut supervisor = ClientSupervisor::new();
            loop {
                tokio::select! {
                    Some(command) = command_rx.recv() => {
                        supervisor.handle_command(command);
                    }
                    Some(frame) = frame_rx.recv() => {
                        // This is where the processing pipeline would be.
                        // For now, we just print the frame ID.
                        println!("Received frame: {:?}", frame.id);
                    }
                    else => break,
                }
            }
        });
        Self {
            frame_tx,
            command_tx,
            frame_handler,
        }
    }

    pub async fn add_client(
        &self,
        reader: Box<dyn FrameReader + Send + Sync>,
        writer: Box<dyn FrameWriter + Send + Sync>,
    ) -> Result<Uuid, AppError> {
        let (action_tx, action_rx) = mpsc::channel(100);
        let visitor = FrameDelegatingVisitor::new(self.frame_tx.clone());
        let mut client = Client::new(reader, writer, Box::new(visitor), action_rx);
        let id = client.id();
        let entry = ClientEntry {
            id,
            client_task: tokio::spawn(async move { client.start().await }),
            action_channel: action_tx,
        };

        let (responder, response_rx) = oneshot::channel();
        self.command_tx
            .send(ClientSupervisorCommand::AddClient { entry, responder })
            .await
            .expect("command channel closed");

        let client_id = response_rx.await.expect("supervisor task died");
        Ok(client_id)
    }

    pub async fn list_clients(&self) -> Vec<Uuid> {
        let (responder, response_rx) = oneshot::channel();
        self.command_tx
            .send(ClientSupervisorCommand::ListClients { responder })
            .await
            .expect("command channel closed");

        response_rx.await.expect("supervisor task died")
    }

    pub async fn remove_client(&self, client_id: Uuid) {
        let (responder, response_rx) = oneshot::channel();
        self.command_tx
            .send(ClientSupervisorCommand::RemoveClient {
                id: client_id,
                responder,
            })
            .await
            .expect("command channel closed");

        response_rx.await.expect("supervisor task died");
    }
}
