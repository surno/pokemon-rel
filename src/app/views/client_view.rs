use crate::app::views::View;
use crate::pipeline::types::EnrichedFrame;
use crate::pipeline::{Scene, State};
use chrono::Utc;
use egui::TextureOptions;
use time::OffsetDateTime;
use uuid::Uuid;
pub struct ClientView {
    client_id: Uuid,
    current_frame: Option<EnrichedFrame>,
    show_frame: bool,
    show_prediction: bool,
    show_game_state: bool,
}

impl ClientView {
    pub fn new(client_id: Uuid, frame: EnrichedFrame) -> Self {
        Self {
            client_id,
            current_frame: Some(frame),
            show_frame: true,
            show_prediction: true,
            show_game_state: true,
        }
    }

    fn draw_frame_info(&self, ui: &mut egui::Ui, frame: &EnrichedFrame) {
        ui.group(|ui| {
            ui.label(format!("Frame Info for Client {}", self.client_id));
            ui.label(format!(
                "Size: {}x{}",
                frame.image.width(),
                frame.image.height()
            ));

            let image = frame.image.to_rgb8();
            ui.label(format!("Pixels: {:?} bytes", image.as_raw().len()));
            ui.label(format!("Timestamp: {:?}", frame.timestamp));
            ui.label(format!(
                "Scene: {:?}",
                frame
                    .state
                    .as_ref()
                    .unwrap_or(&State {
                        scene: Scene::Unknown,
                        player_position: (0.0, 0.0),
                        pokemon_count: 0,
                    })
                    .scene
            ));
        });
    }

    fn draw_game_image(&self, ui: &mut egui::Ui, frame: &EnrichedFrame) {
        ui.group(|ui| {
            ui.label(format!("Game Image for Client {}", self.client_id));

            let image = frame.image.to_rgb8();

            let color_image = egui::ColorImage::from_rgb(
                [image.width() as usize, image.height() as usize],
                image.as_raw().as_slice(),
            );

            let texture_handle =
                ui.ctx()
                    .load_texture("game_frame", color_image, TextureOptions::default());

            ui.image(&texture_handle);
        });
    }
}

impl View for ClientView {
    fn draw(&mut self, ui: &mut egui::Ui) {
        // Main UI
        ui.heading("PokeBot Visualization - Live Debug View");

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_frame, "Show Frame");
            ui.checkbox(&mut self.show_prediction, "Show Prediction");
            ui.checkbox(&mut self.show_game_state, "Show Game State");
        });

        ui.separator();

        if let Some(ref frame) = self.current_frame {
            self.draw_frame_info(ui, frame);
            self.draw_game_image(ui, frame);
        }
    }
}
