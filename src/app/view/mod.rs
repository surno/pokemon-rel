pub mod viewfinder;

pub use viewfinder::ViewFinder;

pub trait View {
    fn ui(&mut self, ui: &mut egui::Ui);
}
