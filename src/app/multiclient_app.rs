use crate::config::Settings;
use crate::emulator::EmulatorClient;
use crate::error::AppError;
use crate::intake::client::manager::{ClientManager, ClientManagerHandle};
use crate::network::server::Server;
use crate::pipeline::{EnrichedFrame, GameAction, services::AIPipelineService};
use tokio::sync::mpsc::error::TryRecvError as MpscTryRecvError;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::app::views::{View, client_view::ClientView};

pub enum UiUpdate {
    ClientList(Vec<Uuid>),
}

pub struct MultiClientApp {
    frame_rx: broadcast::Receiver<EnrichedFrame>,
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
    ai_pipeline_service: AIPipelineService,
    errors: Vec<AppError>,
}

impl MultiClientApp {
    pub fn new(
        frame_rx: broadcast::Receiver<EnrichedFrame>,
        client_manager: ClientManager,
        client_manager_handle: ClientManagerHandle,
        emulator_client: EmulatorClient,
        mut server: Server,
        ai_pipeline_service: AIPipelineService,
    ) -> Self {
        let (ui_update_tx, ui_update_rx) = mpsc::channel::<UiUpdate>(100);
        let server_task = tokio::spawn(async move {
            server.start().await.expect("Server task died");
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
            ai_pipeline_service,
            errors: Vec::new(),
        }
    }

    pub fn start_gui(settings: &Settings) {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(egui::vec2(1280.0, 720.0))
                .with_title("PokeBot Visualization - Multi Client View"),
            ..Default::default()
        };

        let (frame_tx, frame_rx) = broadcast::channel::<EnrichedFrame>(10000);
        let (action_tx, mut _action_rx) = mpsc::channel::<(Uuid, GameAction)>(1000);

        let (client_manager, client_manager_handle) = ClientManager::new(frame_tx.clone());

        let server = Server::new(3344, client_manager_handle.clone());

        let mut emulator_client = EmulatorClient::new(
            1,
            client_manager_handle.clone(),
            settings.emulator.rom_path.clone(),
        );
        emulator_client.start();

        // Create AI pipeline service
        let ai_pipeline_service = AIPipelineService::new(action_tx);

        // Spawn a task to route actions from the AI to the correct client
        let client_manager_handle_clone = client_manager_handle.clone();
        tokio::spawn(async move {
            while let Some((client_id, action)) = _action_rx.recv().await {
                client_manager_handle_clone
                    .send_action_to_client(client_id, action)
                    .await;
            }
        });

        // Spawn a task for the AI pipeline to process frames
        let mut ai_frame_rx = frame_tx.subscribe();
        let mut ai_pipeline_clone = ai_pipeline_service.clone();
        tokio::spawn(async move {
            loop {
                match ai_frame_rx.recv().await {
                    Ok(frame) => {
                        if let Err(e) = ai_pipeline_clone.process_frame(frame).await {
                            error!("AI pipeline failed to process frame: {}", e);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("AI pipeline lagged behind, skipping {} frames", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Frame channel closed, AI pipeline shutting down.");
                        break;
                    }
                }
            }
        });

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
                    ai_pipeline_service,
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

        egui::TopBottomPanel::bottom("error_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Error Log");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for error in self.errors.iter().rev() {
                        ui.label(format!("[ERROR] {}", error));
                    }
                });
            });

        if self.show_frame {
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(selected_client) = &self.selected_client {
                    match self.frame_rx.try_recv() {
                        Ok(frame) => {
                            self.cached_frame = Some(frame);
                        }
                        Err(broadcast::error::TryRecvError::Lagged(n)) => {
                            warn!("UI lagged behind, skipping {} frames", n);
                        }
                        Err(broadcast::error::TryRecvError::Closed) => {
                            let err = AppError::Ui(
                                "Frame receiver disconnected. This can happen during shutdown."
                                    .to_string(),
                            );
                            self.errors.push(err);
                        }
                        Err(broadcast::error::TryRecvError::Empty) => {}
                    }

                    // Display AI statistics
                    ui.heading("AI Pipeline Statistics");
                    let stats = self.ai_pipeline_service.get_stats();
                    ui.label(format!(
                        "Frames Processed: {}",
                        stats.total_frames_processed
                    ));
                    ui.label(format!("Decisions Made: {}", stats.total_decisions_made));
                    ui.label(format!(
                        "Average Confidence: {:.2}",
                        stats.average_confidence
                    ));

                    if let Some(last_time) = stats.last_decision_time {
                        ui.label(format!(
                            "Last Decision: {:?} ago",
                            std::time::Instant::now().duration_since(last_time)
                        ));
                    }

                    ui.separator();

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
