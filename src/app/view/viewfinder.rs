use std::sync::Arc;

pub struct ViewFinder {
    image: Option<(Arc<egui::ColorImage>, egui::TextureHandle)>,
}

impl crate::app::view::View for ViewFinder {
    fn ui(&mut self, ui: &mut egui::Ui) {
        // draw a rectangle to display the nintendo 3ds dual screen from the incoming frame
        let rect =
            egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(100.0, 100.0));
        ui.allocate_rect(rect, egui::Sense::click());
    }
}

impl ViewFinder {
    pub fn new() -> Self {
        Self { image: None }
    }
}
