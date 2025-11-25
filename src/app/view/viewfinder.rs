use crate::pipeline::context::frame_context::FrameContext;
use crate::pipeline::context::state::AnalyzedState;
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
            let width = image.width() as usize;
            let height = image.height() as usize;
            
            // Validate dimensions to prevent crashes
            if width == 0 || height == 0 {
                tracing::warn!("Invalid image dimensions: {}x{}", width, height);
                return;
            }
            
            // Convert image to RGBA format with proper buffer management
            // Store the RGBA image to ensure it lives long enough
            let rgba_image = image.to_rgba8();
            let expected_size = width * height * 4;
            let rgba_buffer = rgba_image.into_raw();
            
            // Validate buffer size matches expected dimensions
            if rgba_buffer.len() != expected_size {
                tracing::error!(
                    "Buffer size mismatch: expected {} bytes, got {} bytes for {}x{} image",
                    expected_size,
                    rgba_buffer.len(),
                    width,
                    height
                );
                return;
            }
            
            let color_image = ColorImage::from_rgba_unmultiplied(
                [width, height],
                &rgba_buffer,
            );
            
            // Check if we need to recreate the texture (e.g., if dimensions changed)
            let needs_recreate = self.image.as_ref().map_or(true, |tex| {
                let tex_size = tex.size();
                tex_size[0] != width || tex_size[1] != height
            });
            
            if needs_recreate {
                self.image = Some(ui.ctx().load_texture(
                    "Emulator Frame",
                    color_image,
                    TextureOptions::default(),
                ));
            } else {
                // Update existing texture - this is more efficient than recreating
                if let Some(tex) = self.image.as_mut() {
                    tex.set(color_image, TextureOptions::default());
                }
            }

            if let Some(tex) = self.image.as_ref() {
                ui.image(ImageSource::Texture(SizedTexture::new(
                    tex.id(),
                    Vec2::new(width as f32, height as f32),
                )));
            }
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
