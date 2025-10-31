use serde::Deserialize;

pub struct Configuration {
    pub rom_path: String,
    pub frame_buffer_size: usize,
    pub action_buffer_size: usize,
    pub enable_metrics: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            rom_path: String::new(),
            frame_buffer_size: 60,
            action_buffer_size: 10,
            enable_metrics: false,
        }
    }
}
