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
        // Main UI
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("PokeBot Visualization - Multi Client View");

            if let Some(client_id) = self.selected_client {
                let mut client_view = ClientView::new(
                    client_id,
                    self.client_frames.get(&client_id).unwrap().clone(),
                );
                client_view.draw(ui);
            }
        });
    }
}
