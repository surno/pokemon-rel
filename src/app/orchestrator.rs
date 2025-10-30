use tokio::sync::mpsc;

use crate::{emulator::EmulatorClient, error::AppError};

pub struct AppOrchestrator {
    emulator_client: EmulatorClient,
}

impl AppOrchestrator {
    pub(crate) fn new(emulator_client: EmulatorClient) -> Self {
        Self { emulator_client }
    }

    pub fn stop(&mut self) {
        self.emulator_client.stop();
    }
}

pub struct AppOrchestratorBuilder {
    rom_path: Option<String>,
}

impl AppOrchestratorBuilder {
    pub fn new() -> Self {
        Self { rom_path: None }
    }

    pub fn add_rom_path(mut self, rom_path: String) -> Self {
        self.rom_path = Some(rom_path);
        self
    }

    pub fn build(self) -> Result<AppOrchestrator, AppError> {
        let (action_tx, action_rx) = mpsc::channel(100);
        let (frame_tx, frame_rx) = mpsc::channel(100);
        match self.rom_path {
            Some(rom_path) => {
                let emulator_client = EmulatorClient::new(action_rx, frame_tx, rom_path);
                Ok(AppOrchestrator::new(emulator_client))
            }
            None => Err(AppError::Client("Emulator client not provided".to_string())),
        }
    }
}
