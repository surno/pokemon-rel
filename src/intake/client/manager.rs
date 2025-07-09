use std::fs;

use crate::{
    error::AppError,
    intake::{
        client::{
            Client, ClientSupervisor,
            supervisor::{ClientEntry, ClientSupervisorCommand},
        },
        frame::{reader::FrameReader, visitor::FrameDelegatingVisitor, writer::FrameWriter},
    },
    pipeline::{
        EnrichedFrame, controller::AppController, services::image::SceneAnnotationServiceBuilder,
        types::Scene,
    },
};

use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Clone)]
pub struct ClientManagerHandle {
    command_tx: mpsc::Sender<ClientSupervisorCommand>,
    frame_tx: mpsc::Sender<EnrichedFrame>,
}

impl ClientManagerHandle {
    pub fn new(
        command_tx: mpsc::Sender<ClientSupervisorCommand>,
        frame_tx: mpsc::Sender<EnrichedFrame>,
    ) -> Self {
        Self {
            command_tx,
            frame_tx,
        }
    }

    pub async fn add_client(
        &self,
        reader: Box<dyn FrameReader + Send + Sync>,
        writer: Box<dyn FrameWriter + Send + Sync>,
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
}

pub struct ClientManager {
    frame_handler: JoinHandle<()>,
    client_handler: JoinHandle<()>,
}

impl ClientManager {
    pub fn new(broadcast_tx: mpsc::Sender<EnrichedFrame>) -> (Self, ClientManagerHandle) {
        // Generate a channel for receiving frames from the client
        let (frame_tx, frame_rx) = mpsc::channel::<EnrichedFrame>(100);
        let (command_tx, mut command_rx) = mpsc::channel::<ClientSupervisorCommand>(100);
        let command_tx_clone = command_tx.clone();
        let frame_handler = tokio::spawn(async move {
            // load assets/intro_frame_hashes.txt
            let intro_frame_hashes = fs::read_to_string(
                "/Users/tony/Projects/pokemon-shiny/assets/intro_frames_hashes.txt",
            )
            .expect("Failed to read intro frame hashes");
            let intro_frame_hashes = intro_frame_hashes.lines().map(|s| s.to_string()).collect();

            let main_menu_frame_hashes = fs::read_to_string(
                "/Users/tony/Projects/pokemon-shiny/assets/main_menu_frames_hashes.txt",
            )
            .expect("Failed to read main menu frame hashes");
            let main_menu_frame_hashes: Vec<String> = main_menu_frame_hashes
                .lines()
                .map(|s| s.to_string())
                .collect();

            let scene_annotation_service = SceneAnnotationServiceBuilder::new(1000, 0.01)
                .with_scene(Scene::Intro, intro_frame_hashes)
                .with_scene(Scene::MainMenu, main_menu_frame_hashes)
                .build();

            let mut controller = AppController::new(
                frame_rx,
                broadcast_tx,
                command_tx_clone,
                scene_annotation_service,
            );
            if let Err(e) = controller.run().await {
                error!("Client manager frame handler task died: {:?}", e);
            }
        });
        let client_handler = tokio::spawn(async move {
            let mut supervisor = ClientSupervisor::new();
            loop {
                tokio::select! {
                    Some(command) = command_rx.recv() => {
                        supervisor.handle_command(command);
                    }
                    else => {
                        error!("Client manager frame handler task died");
                        break;
                    },
                }
            }
        });
        (
            Self {
                frame_handler,
                client_handler,
            },
            ClientManagerHandle {
                command_tx,
                frame_tx,
            },
        )
    }
}
