use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

use crate::app::views::{View, client_view::ClientView};
use crate::intake::client::manager::{ClientHandle, ClientManager};

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
    client_manager: ClientManager,
    fps_tracker: HashMap<Uuid, FpsTracker>,
    show_frame: bool,
    selected_client: Option<Uuid>,
    selected_client_handle: Option<ClientHandle>,
}

impl MultiClientApp {
    pub fn new(client_manager: ClientManager) -> Self {
        Self {
            client_manager,
            show_frame: true,
            fps_tracker: HashMap::new(),
            selected_client: None,
            selected_client_handle: None,
        }
    }

    pub fn start_gui(client_manager: ClientManager) {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(egui::vec2(1280.0, 720.0))
                .with_title("PokeBot Visualization - Multi Client View"),
            ..Default::default()
        };

        let _result = eframe::run_native(
            "PokeBot Visualization - Multi Client View",
            options,
            Box::new(move |_cc| Ok(Box::new(MultiClientApp::new(client_manager.clone())))),
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

                let client_ids = self.client_manager.list_clients();

                egui::ComboBox::from_label("Active Client.")
                    .selected_text(
                        self.selected_client
                            .map(|id| id.to_string())
                            .unwrap_or("None".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for client_id in client_ids {
                            let client_name = format!("Client {}", client_id);
                            ui.selectable_value(
                                &mut self.selected_client,
                                Some(client_id),
                                client_name,
                            );
                        }
                    });

                if let Some(client_id) = self.selected_client {
                    self.selected_client_handle = self.client_manager.subscribe(&client_id);
                }
            });

        if self.show_frame {
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(selected_client) = &mut self.selected_client_handle {
                    if let Ok(frame) = selected_client.get_frame() {
                        ui.heading(format!("Detailed View - Client {}", selected_client.id()));
                        let fps_tracker = self.fps_tracker.get_mut(&selected_client.id()).unwrap();
                        let fps = fps_tracker.get_fps();
                        let mut client_view = ClientView::new(selected_client.id(), frame, fps);
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
