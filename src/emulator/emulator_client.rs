use image::{DynamicImage, RgbImage};
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use crate::{error::AppError, pipeline::GameAction};

pub struct EmulatorClient {
    action_rx: Receiver<GameAction>,
    frame_tx: Sender<DynamicImage>,
    rom_path: String,
    id: Uuid,
}

impl EmulatorClient {
    pub fn new(
        action_rx: Receiver<GameAction>,
        frame_tx: Sender<DynamicImage>,
        rom_path: String,
    ) -> Self {
        Self {
            action_rx,
            frame_tx,
            rom_path,
            id: Uuid::new_v4(),
        }
    }

    fn initalize_desmume(
        &mut self,
        rom_path: &str,
        auto_resume: bool,
    ) -> Result<desmume_rs::DeSmuME, AppError> {
        let mut desmume =
            desmume_rs::DeSmuME::init().map_err(|e| AppError::Emulator(e.to_string()))?;
        if let Err(e) = desmume.open(rom_path, auto_resume) {
            let err_msg = format!(
                "Failed to open ROM at path '{}': {:?}. Shutting down emulator task.",
                rom_path, e
            );
            tracing::error!("{}", err_msg);
            return Err(AppError::Emulator(err_msg));
        }
        // Set volume to 0 to avoid audio output, it's annoying and unnecessary.
        desmume.volume_set(0);
        Ok(desmume)
    }

    fn release_key(&mut self, desmume: &mut desmume_rs::DeSmuME) {
        desmume.input_mut().keypad_update(0);
    }

    fn prepare_action(&mut self, action: GameAction, desmume: &mut desmume_rs::DeSmuME) {
        let mask: u16 = match action {
            GameAction::A => 1 << 0,
            GameAction::B => 1 << 1,
            GameAction::Select => 1 << 2,
            GameAction::Start => 1 << 3,
            GameAction::Right => 1 << 4,
            GameAction::Left => 1 << 5,
            GameAction::Up => 1 << 6,
            GameAction::Down => 1 << 7,
            GameAction::R => 1 << 8,
            GameAction::L => 1 << 9,
            GameAction::X => 1 << 10,
            // If GameAction::Y does not exist, map nothing for that slot
            _ => 0,
        };
        if mask != 0 {
            desmume.input_mut().keypad_update(mask);
            tracing::info!("Applied keypad mask {:#018b} for action {:?}", mask, action);
        } else {
            tracing::warn!("No keypad mapping for action {:?}", action);
        }
    }

    fn process_frame(&mut self, desmume: &mut desmume_rs::DeSmuME) {
        let buffer = desmume.display_buffer_as_rgbx();
        let mut new_buffer: Vec<u8> = Vec::with_capacity(buffer.len() / 4 * 3);
        // -- pixel order is B G R A; convert to R G B
        for chunk in buffer.chunks_exact(4) {
            // chunk = [B, G, R, A]
            new_buffer.extend_from_slice(&[chunk[2], chunk[1], chunk[0]]);
        }

        let image = DynamicImage::ImageRgb8(
            RgbImage::from_raw(
                desmume_rs::SCREEN_WIDTH as u32,
                desmume_rs::SCREEN_HEIGHT_BOTH as u32,
                new_buffer,
            )
            .unwrap(),
        );

        match self.frame_tx.try_send(image) {
            Ok(_) => {}
            Err(err) => match err {
                TrySendError::Full(_) => {
                    // Drop frame to keep real-time
                    tracing::warn!("Dropping frame: channel full");
                }
                TrySendError::Closed(_) => {
                    tracing::warn!("Frame channel closed, stopping emulator loop");
                }
            },
        }
    }

    pub fn run(&mut self) {
        tracing::info!("EmulatorClient starting game, with unique id: {}", self.id);

        let desmume = self.initalize_desmume(&self.rom_path.clone(), true);
        match desmume {
            Ok(mut desmume) => {
                while desmume.is_running() {
                    match self.action_rx.try_recv() {
                        Ok(action) => {
                            self.prepare_action(action, &mut desmume);
                        }
                        Err(TryRecvError::Disconnected) => {
                            tracing::error!("Action channel closed, stopping emulator loop");
                            break;
                        }
                        Err(_) => {
                            // No action to process, cycle the emulator and process the frame
                        }
                    }
                    desmume.cycle();
                    self.release_key(&mut desmume);
                    self.process_frame(&mut desmume);
                }
            }
            Err(e) => {
                tracing::error!("Error initializing desmume: {}", e);
            }
        }
    }
}
