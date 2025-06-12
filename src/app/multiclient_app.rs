use std::collections::HashMap;

use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::app::views::View;
use crate::app::views::client_view::ClientView;
use crate::pipeline::types::SharedFrame;

pub struct MultiClientApp {
    client_receiver: HashMap<Uuid, Receiver<SharedFrame>>,
    client_frames: HashMap<Uuid, SharedFrame>,
    selected_client: Option<Uuid>,
    show_overview: bool,
    show_frame: bool,
    show_prediction: bool,
    show_game_state: bool,
}

impl Default for MultiClientApp {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiClientApp {
    pub fn new() -> Self {
        Self {
            client_receiver: HashMap::new(),
            client_frames: HashMap::new(),
            selected_client: None,
            show_overview: false,
            show_frame: true,
            show_prediction: true,
            show_game_state: true,
        }
    }

    pub async fn start_gui() {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(egui::vec2(1280.0, 720.0))
                .with_title("PokeBot Visualization - Multi Client View"),
            ..Default::default()
        };

        let _result = eframe::run_native(
            "PokeBot Visualization - Multi Client View",
            options,
            Box::new(|cc| Ok(Box::new(MultiClientApp::default()))),
        );
    }

    pub fn add_client(&mut self, client_id: Uuid, receiver: Receiver<SharedFrame>) {
        self.client_receiver.insert(client_id, receiver);
        if self.selected_client.is_none() {
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

impl eframe::App for MultiClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Upfate frames from all clients
        for (client_id, receiver) in self.client_receiver.iter_mut() {
            if let Ok(frame) = receiver.try_recv() {
                self.client_frames.insert(*client_id, frame);
            }
        }

        // Main UI
        egui::TopBottomPanel::top("Client Selector")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("PokeBot Visualization - Multi Client View");
                ui.separator();

                ui.checkbox(&mut self.show_overview, "Show Overview");

                egui::ComboBox::from_label("Active Client.")
                    .selected_text(
                        self.selected_client
                            .map(|id| id.to_string())
                            .unwrap_or("None".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for client_id in self.client_receiver.keys() {
                            let client_name = format!("Client {}", client_id);
                            ui.selectable_value(
                                &mut self.selected_client,
                                Some(*client_id),
                                client_name,
                            );
                        }
                    });
            });

        if self.show_overview {
            egui::SidePanel::left("overview")
                .resizable(true)
                .show(ctx, |ui| {
                    ui.heading("Overview");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (client_id, frame) in self.client_frames.iter() {
                            ui.group(|ui| {
                                let is_selected = self.selected_client == Some(*client_id);
                                let client_name = format!("Client {}", client_id);
                                if ui.button(&client_name).clicked() {
                                    self.selected_client = Some(*client_id);
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
                if let Some(selected_client) = self.selected_client {
                    if let Some(frame) = self.client_frames.get(&selected_client) {
                        ui.heading(format!("Detailed View - Client {}", selected_client));
                        let mut client_view = ClientView::new(selected_client, frame.clone());
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
