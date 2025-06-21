use crate::app::views::View;
use crate::pipeline::types::EnrichedFrame;
use egui::TextureOptions;
use time::OffsetDateTime;
use uuid::Uuid;
pub struct ClientView {
    client_id: Uuid,
    current_frame: Option<EnrichedFrame>,
    show_frame: bool,
    show_prediction: bool,
    show_game_state: bool,
    fps: f32,
}

impl ClientView {
    pub fn new(client_id: Uuid, frame: EnrichedFrame, fps: f32) -> Self {
        Self {
            client_id,
            current_frame: Some(frame),
            show_frame: true,
            show_prediction: true,
            show_game_state: true,
            fps,
        }
    }

    fn draw_frame_info(&self, ui: &mut egui::Ui, frame: &EnrichedFrame) {
        ui.group(|ui| {
            ui.label(format!("Frame Info for Client {}", self.client_id));
            ui.label(format!(
                "Size: {}x{}",
                frame.raw.image.width(),
                frame.raw.image.height()
            ));
            ui.label(format!(
                "Pixels: {:?} bytes",
                frame.raw.image.as_rgb8().unwrap().as_raw().len()
            ));
            ui.label(format!(
                "Timestamp: {:?}",
                OffsetDateTime::from_unix_timestamp(frame.raw.timestamp as i64).unwrap()
            ));
            ui.label(format!("FPS: {:.1}", self.fps));
        });
    }

    fn draw_game_image(&self, ui: &mut egui::Ui, frame: &EnrichedFrame) {
        ui.group(|ui| {
            ui.label(format!("Game Image for Client {}", self.client_id));

            let image = frame.raw.image.as_rgb8().unwrap();

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

    fn draw_prediction_info(&self, ui: &mut egui::Ui, frame: &EnrichedFrame) {
        ui.group(|ui| {
            ui.label(format!("Prediction Info for Client {}", self.client_id));
            match frame.ml_prediction.as_ref() {
                Some(prediction) => {
                    ui.label(format!("Confidence: {:?}", prediction.confidence * 100.0));
                }
                None => {
                    ui.label("No prediction available");
                }
            }

            match frame.ml_prediction.as_ref() {
                Some(prediction) => {
                    ui.label(format!("Value Estimate: {:?}", prediction.value_estimate));

                    for (i, &prob) in prediction.action_probabilities.iter().enumerate() {
                        let button_match = match i {
                            0 => "A",
                            1 => "B",
                            2 => "Up",
                            3 => "Down",
                            4 => "Left",
                            5 => "Right",
                            _ => "Unknown",
                        };

                        ui.horizontal(|ui| {
                            ui.label(format!("{}: {:?}%", button_match, prob * 100.0));
                            ui.add(egui::ProgressBar::new(prob).desired_width(100.0));
                            ui.label(format!("{:.2}%", prob * 100.0));
                        });
                    }
                }
                None => {
                    ui.label("No prediction available");
                }
            }
        });
    }

    fn draw_game_state_info(&self, ui: &mut egui::Ui, frame: &EnrichedFrame) {
        ui.group(|ui| {
            ui.label(format!("Game State Info for Client {}", self.client_id));

            match frame.game_action.as_ref() {
                Some(action) => {
                    ui.label(format!("Action: {:?}", action.action));
                }
                None => {
                    ui.label("No action available");
                }
            }
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

            if self.show_prediction {
                self.draw_prediction_info(ui, frame);
            }

            if self.show_game_state {
                self.draw_game_state_info(ui, frame);
            }
        }
    }
}
