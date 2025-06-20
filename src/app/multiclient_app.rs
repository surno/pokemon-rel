use std::time::SystemTime;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::app::views::{View, client_view::ClientView};
use crate::intake::client::client_manager::{ClientManagerTrait, FrameReaderClientManager};

struct FpsTracker {
    last_timestamp: SystemTime,
    frame_count: usize,
    last_fps: f32,
}

impl FpsTracker {
    pub fn new() -> Self {
        Self {
            last_timestamp: SystemTime::now(),
            frame_count: 0,
            last_fps: 0.0,
        }
    }

    pub fn add_frame(&mut self) {
        self.frame_count += 1;
    }

    pub fn get_fps(&mut self) -> f32 {
        let now = SystemTime::now();
        let elapsed_time = now.duration_since(self.last_timestamp).unwrap();
        // every 10 seconds, reset the frame count
        if elapsed_time.as_secs() > 1 {
            self.last_fps = self.frame_count as f32 / elapsed_time.as_secs() as f32;
            self.frame_count = 0;
            self.last_timestamp = now;
        }
        self.last_fps
    }
}

pub struct MultiClientApp {
    client_manager: Arc<RwLock<FrameReaderClientManager>>,
    fps_tracker: HashMap<Uuid, FpsTracker>,
    show_overview: bool,
    show_frame: bool,
    show_prediction: bool,
    show_game_state: bool,
}

impl Default for MultiClientApp {
    fn default() -> Self {
        Self::new(Arc::new(RwLock::new(FrameReaderClientManager::new())))
    }
}

impl MultiClientApp {
    pub fn new(client_manager: Arc<RwLock<FrameReaderClientManager>>) -> Self {
        Self {
            client_manager,
            show_overview: false,
            show_frame: true,
            show_prediction: true,
            show_game_state: true,
            fps_tracker: HashMap::new(),
        }
    }

    pub fn start_gui(client_manager: Arc<RwLock<FrameReaderClientManager>>) {
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
        // Main UI
        egui::TopBottomPanel::top("Client Selector")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("PokeBot Visualization - Multi Client View");
                ui.separator();

                ui.checkbox(&mut self.show_overview, "Show Overview");

                let mut selected_client = self.client_manager.blocking_read().get_selected_client();
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

                if let Some(client_id) = selected_client {
                    self.client_manager
                        .blocking_write()
                        .set_selected_client(client_id);
                }
            });

        if self.show_overview {
            egui::SidePanel::left("overview")
                .resizable(true)
                .show(ctx, |ui| {
                    ui.heading("Overview");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let mut client_manager = self.client_manager.blocking_write();
                        for client_id in client_manager.get_clients() {
                            let Some(frame) = client_manager.get_frame_from_client(client_id)
                            else {
                                continue;
                            };
                            ui.group(|ui| {
                                let is_selected =
                                    self.client_manager.blocking_read().get_selected_client()
                                        == Some(client_id);
                                let client_name = format!("Client {}", client_id);
                                if ui.button(&client_name).clicked() {
                                    self.client_manager
                                        .blocking_write()
                                        .set_selected_client(client_id);
                                }

                                // Mini preview
                                ui.label(format!(
                                    "Client {}x{}",
                                    frame.raw.width, frame.raw.height
                                ));

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
                if let Some(selected_client) =
                    self.client_manager.blocking_read().get_selected_client()
                {
                    let mut client_manager = self.client_manager.blocking_write();
                    if let Some(frame) = client_manager.get_frame_from_client(selected_client) {
                        ui.heading(format!("Detailed View - Client {}", selected_client));
                        let fps_tracker = self.fps_tracker.get_mut(&selected_client).unwrap();
                        let fps = fps_tracker.get_fps();
                        let mut client_view = ClientView::new(selected_client, frame, fps);
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
