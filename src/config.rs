use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub emulator: EmulatorSettings,
}

#[derive(Debug, Deserialize)]
pub struct EmulatorSettings {
    pub rom_path: String,
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {
        let s = config::Config::builder()
            .add_source(config::File::with_name("config/default"))
            .build()?;
        s.try_deserialize()
    }
}
