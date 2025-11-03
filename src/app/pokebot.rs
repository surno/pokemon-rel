use crate::app::view::{View, ViewFinder};
use crate::pipeline::context::frame_context::FrameContext;
use crate::pipeline::context::state::AnalyzedState;
use tokio::sync::watch::Receiver;

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
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        frame_rx: Receiver<Option<FrameContext<AnalyzedState>>>,
    ) -> Self {
        Self {
            viewfinder: ViewFinder::new(frame_rx),
        }
    }
}
