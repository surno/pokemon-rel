use std::sync::Arc;

use crate::pipeline::context::frame_context::FrameContext;
use crate::pipeline::context::state::AnalyzedState;
use eframe::glow::Texture;
use egui::{ColorImage, ImageSource, TextureOptions, Vec2, load::SizedTexture};
use tokio::sync::watch::Receiver;

pub struct ViewFinder {
    frame_rx: Receiver<Option<FrameContext<AnalyzedState>>>,
    image: Option<egui::TextureHandle>,
}

impl crate::app::view::View for ViewFinder {
    fn ui(&mut self, ui: &mut egui::Ui) {
        // draw a rectangle to display the nintendo 3ds dual screen from the incoming frame
        let rect =
            egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(100.0, 100.0));
        ui.allocate_rect(rect, egui::Sense::click());
        let frame = self.frame_rx.borrow_and_update();
        if let Some(frame) = frame.as_ref() {
            let image = frame.frame().image();
            let color_image = ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                &image.to_rgba8().into_raw(),
            );
            self.image = Some(egui::Context::load_texture(
                ui.ctx(),
                "Emulator Frame",
                color_image,
                TextureOptions::default(),
            ));
            ui.image(ImageSource::Texture(SizedTexture::new(
                self.image.as_ref().unwrap().id(),
                Vec2::new(image.width() as f32, image.height() as f32),
            )));
        }
    }
}

impl ViewFinder {
    pub fn new(frame_rx: Receiver<Option<FrameContext<AnalyzedState>>>) -> Self {
        Self {
            frame_rx,
            image: None,
        }
    }
}
