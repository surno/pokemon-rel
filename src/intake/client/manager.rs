use std::fs;

use crate::{
    app::controller::app_controller::AppController,
    error::AppError,
    intake::{
        client::{
            Client,
            supervisor::{ClientEntry, ClientSupervisor, ClientSupervisorCommand},
        },
        frame::{
            reader::FrameReader,
            visitor::{FrameDelegatingVisitor, FrameVisitor},
            writer::FramedWriter,
        },
    },
    pipeline::{EnrichedFrame, GameAction},
};

use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Clone)]
pub struct ClientManagerHandle {
    command_tx: mpsc::Sender<ClientSupervisorCommand>,
    frame_tx: broadcast::Sender<EnrichedFrame>,
}

impl ClientManagerHandle {
    pub fn new(
        command_tx: mpsc::Sender<ClientSupervisorCommand>,
        frame_tx: broadcast::Sender<EnrichedFrame>,
    ) -> Self {
        Self {
            command_tx,
            frame_tx,
        }
    }

    pub async fn add_client(
        &self,
        reader: Box<dyn FrameReader + Send + Sync>,
        writer: Box<dyn FramedWriter + Send + Sync>,
    ) -> Result<Uuid, AppError> {
        debug!("Adding client");
        let (action_tx, action_rx) = mpsc::channel(100);
        let visitor = FrameDelegatingVisitor::new(self.frame_tx.clone());
        let mut client = Client::new(reader, writer, Box::new(visitor), action_rx);
        let id = client.id();
        let entry = ClientEntry {
            id,
            client_task: tokio::spawn(async move {
                debug!("Client {:?} starting thread", id);
                client
                    .start()
                    .await
                    .map_err(|e| AppError::Client(e.to_string()))
            }),
            action_channel: action_tx,
        };

        let (responder, response_rx) = oneshot::channel();
        self.command_tx
            .send(ClientSupervisorCommand::AddClient { entry, responder })
            .await
            .expect("command channel closed");

        let client_id = response_rx.await.expect("supervisor task died");
        debug!("Client {:?} added and started", client_id);
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

    pub async fn send_action_to_client(&self, client_id: Uuid, action: GameAction) {
        if let Err(e) = self
            .command_tx
            .send(ClientSupervisorCommand::SendAction {
                id: client_id,
                action,
            })
            .await
        {
            error!("Failed to send action command to supervisor: {}", e);
        }
    }
}

pub struct ClientManager {
    frame_handler: JoinHandle<()>,
    client_handler: JoinHandle<()>,
}

impl ClientManager {
    pub fn new(frame_tx: broadcast::Sender<EnrichedFrame>) -> (Self, ClientManagerHandle) {
        let (command_tx, mut command_rx) = mpsc::channel::<ClientSupervisorCommand>(100);
        let command_tx_clone = command_tx.clone();
        let frame_tx_clone = frame_tx.clone();

        let client_handler = tokio::spawn(async move {
            let mut supervisor = ClientSupervisor::new();
            loop {
                if let Some(command) = command_rx.recv().await {
                    supervisor.handle_command(command);
                }
            }
        });

        let client_manager = ClientManager {
            frame_handler: tokio::spawn(async {}), // Placeholder
            client_handler,
        };

        let handle = ClientManagerHandle::new(command_tx_clone, frame_tx_clone);

        (client_manager, handle)
    }
}
