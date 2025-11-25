pub struct Configuration {
    pub rom_path: String,
    pub frame_buffer_size: usize,
    pub action_buffer_size: usize,
    pub enable_metrics: bool,
    /// Renderer selection: "auto", "metal", or "software"
    /// - "auto": Use Metal on macOS, software on other platforms
    /// - "metal": Force Metal renderer (macOS only, falls back to software if unavailable)
    /// - "software": Force software rasterizer
    pub renderer: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            rom_path: String::new(),
            frame_buffer_size: 60,
            action_buffer_size: 10,
            enable_metrics: false,
            renderer: "auto".to_string(),
        }
    }
}
