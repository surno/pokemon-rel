use crate::app::views::View;
use crate::pipeline::types::{RawFrame, SharedFrame};
use egui::TextureOptions;
use image::{ImageBuffer, Rgb};
use uuid::Uuid;

pub struct ClientView {
    client_id: Uuid,
    current_frame: Option<SharedFrame>,
    show_frame: bool,
    show_prediction: bool,
    show_game_state: bool,
}

impl ClientView {
    pub fn new(client_id: Uuid, frame: SharedFrame) -> Self {
        Self {
            client_id,
            current_frame: Some(frame),
            show_frame: true,
            show_prediction: true,
            show_game_state: true,
        }
    }

    fn convert_pixels_to_image(&self, frame: &RawFrame) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
        // Nintendo DS is 256x384
        let width = frame.width;
        let height = frame.height;

        ImageBuffer::from_fn(width, height, |x, y| {
            let idx = ((y * width + x) * 3) as usize;
            // the first 3 bytes are the rgb values
            let r = frame.pixels.get(idx).copied().unwrap_or(0);
            let g = frame.pixels.get(idx + 1).copied().unwrap_or(0);
            let b = frame.pixels.get(idx + 2).copied().unwrap_or(0);
            Rgb([r, g, b])
        })
    }

    fn draw_frame_info(&self, ui: &mut egui::Ui, frame: &SharedFrame) {
        ui.group(|ui| {
            ui.label(format!("Frame Info for Client {}", self.client_id));
            ui.label(format!("Size: {}x{}", frame.raw.width, frame.raw.height));
            ui.label(format!("Pixels: {:?} bytes", frame.raw.pixels.len()));
            ui.label(format!("Timestamp: {:?}", frame.raw.timestamp));
        });
    }

    fn draw_game_image(&self, ui: &mut egui::Ui, frame: &SharedFrame) {
        ui.group(|ui| {
            ui.label(format!("Game Image for Client {}", self.client_id));

            let image = self.convert_pixels_to_image(&frame.raw);

            let color_image = egui::ColorImage::from_rgb(
                [image.width() as usize, image.height() as usize],
                image.into_raw().as_slice(),
            );

            let texture_handle =
                ui.ctx()
                    .load_texture("game_frame", color_image, TextureOptions::default());

            ui.image(&texture_handle);
        });
    }

    fn draw_prediction_info(&self, ui: &mut egui::Ui, frame: &SharedFrame) {
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

    fn draw_game_state_info(&self, ui: &mut egui::Ui, frame: &SharedFrame) {
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
