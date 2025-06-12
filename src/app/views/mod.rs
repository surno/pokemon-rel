pub mod client_view;

pub trait View {
    fn draw(&mut self, ui: &mut egui::Ui);
}
