use crate::app::view::{View, ViewFinder};

pub struct PokeBot {
    viewfinder: ViewFinder,
}

impl eframe::App for PokeBot {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.viewfinder.ui(ui);
        });
    }
}

impl PokeBot {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            viewfinder: ViewFinder::new(),
        }
    }
}
