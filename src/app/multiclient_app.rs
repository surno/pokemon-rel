use image::DynamicImage;
use tokio::sync::broadcast::error::TryRecvError as BroadcastTryRecvError;
use tokio::sync::mpsc::error::TryRecvError as MpscTryRecvError;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::app::views::{View, client_view::ClientView};
use crate::emulator::EmulatorClient;
use crate::intake::client::manager::{ClientManager, ClientManagerHandle};
use crate::network::server::Server;
use crate::pipeline::EnrichedFrame;
use tracing::{debug, error};

pub enum UiUpdate {
    ClientList(Vec<Uuid>),
}

pub struct MultiClientApp {
    frame_rx: mpsc::Receiver<EnrichedFrame>,
    show_frame: bool,
    selected_client: Option<Uuid>,
    emulator_client: EmulatorClient,
    client_manager: ClientManager,
    client_manager_handle: ClientManagerHandle,
    server_task: JoinHandle<()>,
    ui_update_rx: mpsc::Receiver<UiUpdate>,
    ui_update_tx: mpsc::Sender<UiUpdate>,
    client_id_task: JoinHandle<()>,
    client_ids: Vec<Uuid>,
    cached_frame: Option<EnrichedFrame>,
}

impl MultiClientApp {
    pub fn new(
        frame_rx: mpsc::Receiver<EnrichedFrame>,
        client_manager: ClientManager,
        client_manager_handle: ClientManagerHandle,
        emulator_client: EmulatorClient,
        mut server: Server,
    ) -> Self {
        let (ui_update_tx, ui_update_rx) = mpsc::channel::<UiUpdate>(100);
        let server_task = tokio::spawn(async move {
            server.start().await.unwrap();
        });

        let clone_handle = client_manager_handle.clone();
        let clone_tx = ui_update_tx.clone();

        let client_id_task = tokio::spawn(async move {
            loop {
                let client_ids = clone_handle.list_clients().await;
                debug!("Client IDs to update: {:?}", client_ids);
                match clone_tx.send(UiUpdate::ClientList(client_ids)).await {
                    Ok(_) => {
                        debug!("Client list update sent");
                    }
                    Err(e) => {
                        error!("Error sending client list update: {:?}", e);
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        Self {
            frame_rx,
            show_frame: true,
            selected_client: None,
            emulator_client,
            client_manager,
            client_manager_handle,
            server_task,
            ui_update_rx,
            ui_update_tx,
            client_id_task,
            client_ids: Vec::new(),
            cached_frame: None,
        }
    }

    pub fn start_gui() {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(egui::vec2(1280.0, 720.0))
                .with_title("PokeBot Visualization - Multi Client View"),
            ..Default::default()
        };

        let (frame_tx, frame_rx) = mpsc::channel::<EnrichedFrame>(10000);
        let (client_manager, client_manager_handle) = ClientManager::new(frame_tx);

        let server = Server::new(3344, client_manager_handle.clone());

        let mut emulator_client = EmulatorClient::new(1, client_manager_handle.clone());
        emulator_client.start();

        let _result = eframe::run_native(
            "PokeBot Visualization - Multi Client View",
            options,
            Box::new(move |_cc| {
                Ok(Box::new(MultiClientApp::new(
                    frame_rx,
                    client_manager,
                    client_manager_handle,
                    emulator_client,
                    server,
                )))
            }),
        );
    }
}

impl eframe::App for MultiClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.ui_update_rx.try_recv() {
            Ok(UiUpdate::ClientList(client_ids)) => {
                self.client_ids = client_ids;
            }
            Err(MpscTryRecvError::Empty) => {}
            Err(MpscTryRecvError::Disconnected) => {
                error!("Client list update receiver disconnected");
            }
        };
        // Main UI
        egui::TopBottomPanel::top("Client Selector")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("PokeBot Visualization - Multi Client View");
                ui.separator();

                egui::ComboBox::from_label("Active Client.")
                    .selected_text(
                        self.selected_client
                            .map(|id| id.to_string())
                            .unwrap_or("None".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for client_id in &self.client_ids {
                            let client_name = format!("Client {}", client_id);
                            ui.selectable_value(
                                &mut self.selected_client,
                                Some(*client_id),
                                client_name,
                            );
                        }
                    });
            });

        if self.show_frame {
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(selected_client) = &self.selected_client {
                    let frame = self.frame_rx.try_recv();
                    match frame {
                        Ok(frame) => {
                            self.cached_frame = Some(frame);
                        }
                        Err(MpscTryRecvError::Empty) => {
                            // debug!("No frame received from client: {:?}", selected_client);
                        }
                        Err(MpscTryRecvError::Disconnected) => {
                            error!("Frame receiver disconnected");
                        }
                    }
                    if let Some(frame) = &self.cached_frame {
                        ui.heading(format!("Detailed View - Client {}", selected_client));
                        let mut client_view = ClientView::new(*selected_client, frame.clone());
                        client_view.draw(ui);
                    }
                } else {
                    ui.heading("No client selected");
                }
            });
        }
        ctx.request_repaint();
    }
}
