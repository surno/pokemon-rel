use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::app::views::{View, client_view::ClientView};
use crate::network::client::client_manager::ClientManager;
use tracing::{debug, info};

struct FpsTracker {
    frame_timestamps: VecDeque<u64>,
    max_frames: usize,
    last_fps: f32,
}

impl FpsTracker {
    pub fn new() -> Self {
        Self {
            frame_timestamps: VecDeque::new(),
            max_frames: 1000,
            last_fps: 0.0,
        }
    }

    pub fn add_frame(&mut self, timestamp: u64) {
        self.frame_timestamps.push_back(timestamp);
        while self.frame_timestamps.len() > self.max_frames {
            self.frame_timestamps.pop_front();
        }

        if self.frame_timestamps.len() >= 2 {
            let oldest_timestamp = self.frame_timestamps.front().unwrap();
            let newest_timestamp = self.frame_timestamps.back().unwrap();
            let elapsed_time = newest_timestamp - oldest_timestamp;

            if elapsed_time > 0 {
                self.last_fps = (self.frame_timestamps.len() - 1) as f32 / elapsed_time as f32;
            }
        }
    }

    pub fn get_fps(&self) -> f32 {
        self.last_fps
    }
}

pub struct MultiClientApp {
    client_manager: Arc<RwLock<ClientManager>>,
    fps_tracker: HashMap<Uuid, FpsTracker>,
    show_overview: bool,
    show_frame: bool,
    show_prediction: bool,
    show_game_state: bool,
}

impl Default for MultiClientApp {
    fn default() -> Self {
        Self::new(Arc::new(RwLock::new(ClientManager::new())))
    }
}

impl MultiClientApp {
    pub fn new(client_manager: Arc<RwLock<ClientManager>>) -> Self {
        Self {
            client_manager,
            show_overview: false,
            show_frame: true,
            show_prediction: true,
            show_game_state: true,
            fps_tracker: HashMap::new(),
        }
    }

    pub fn start_gui(client_manager: Arc<RwLock<ClientManager>>) {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(egui::vec2(1280.0, 720.0))
                .with_title("PokeBot Visualization - Multi Client View"),
            ..Default::default()
        };

        let _result = eframe::run_native(
            "PokeBot Visualization - Multi Client View",
            options,
            Box::new(move |_cc| Ok(Box::new(MultiClientApp::new(client_manager)))),
        );
    }
}

impl eframe::App for MultiClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Upfate frames from all clients
        {
            let mut client_manager = self.client_manager.blocking_write();
            let mut new_frames = Vec::new();
            for (client_id, receiver) in client_manager.client_receiver.iter_mut() {
                let fps_tracker = self
                    .fps_tracker
                    .entry(*client_id)
                    .or_insert(FpsTracker::new());
                debug!("Trying to receive frame from client {}", client_id);
                if let Ok(frame) = receiver.try_recv() {
                    debug!("Received frame from client {}", client_id);
                    fps_tracker.add_frame(frame.raw.timestamp);
                    new_frames.push((*client_id, frame));
                }
            }
            for (client_id, frame) in new_frames {
                client_manager.client_frames.insert(client_id, frame);
            }
        }

        // Main UI
        egui::TopBottomPanel::top("Client Selector")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("PokeBot Visualization - Multi Client View");
                ui.separator();

                ui.checkbox(&mut self.show_overview, "Show Overview");

                let mut selected_client = self.client_manager.blocking_read().selected_client;
                let client_ids: Vec<_> = self
                    .client_manager
                    .blocking_read()
                    .client_receiver
                    .keys()
                    .cloned()
                    .collect();

                egui::ComboBox::from_label("Active Client.")
                    .selected_text(
                        selected_client
                            .map(|id| id.to_string())
                            .unwrap_or("None".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for client_id in client_ids {
                            let client_name = format!("Client {}", client_id);
                            ui.selectable_value(&mut selected_client, Some(client_id), client_name);
                        }
                    });

                self.client_manager.blocking_write().selected_client = selected_client;
            });

        if self.show_overview {
            egui::SidePanel::left("overview")
                .resizable(true)
                .show(ctx, |ui| {
                    ui.heading("Overview");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (client_id, frame) in
                            self.client_manager.blocking_read().client_frames.iter()
                        {
                            ui.group(|ui| {
                                let is_selected =
                                    self.client_manager.blocking_read().selected_client
                                        == Some(*client_id);
                                let client_name = format!("Client {}", client_id);
                                if ui.button(&client_name).clicked() {
                                    self.client_manager.blocking_write().selected_client =
                                        Some(*client_id);
                                }

                                // Mini preview
                                ui.label(format!(
                                    "Client {}x{}",
                                    frame.raw.width, frame.raw.height
                                ));
                                if let Some(enriched) = frame.enriched.as_ref() {
                                    let player_position = enriched.game_state.player_position;
                                    ui.label(format!(
                                        "Player Position: {}, {}",
                                        player_position.0, player_position.1
                                    ));
                                }

                                if let Some(prediction) = frame.ml_prediction.as_ref() {
                                    ui.label(format!("Prediction: {:?}", prediction.confidence));
                                }

                                if is_selected {
                                    ui.colored_label(egui::Color32::GREEN, "Selected");
                                }
                            });
                        }
                    });
                });
        }

        if self.show_frame {
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(selected_client) = self.client_manager.blocking_read().selected_client {
                    if let Some(frame) = self
                        .client_manager
                        .blocking_read()
                        .client_frames
                        .get(&selected_client)
                    {
                        ui.heading(format!("Detailed View - Client {}", selected_client));
                        let fps = self.fps_tracker.get(&selected_client).unwrap().get_fps();
                        let mut client_view = ClientView::new(selected_client, frame.clone(), fps);
                        client_view.draw(ui);
                    } else {
                        ui.heading("No frame available... waiting for frame from client");
                    }
                } else {
                    ui.heading("No client selected");
                }
            });
        }
        ctx.request_repaint();
    }
}
