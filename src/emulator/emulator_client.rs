use image::{DynamicImage, RgbImage};
use tokio::{
    sync::{mpsc, mpsc::error::TrySendError},
    task::JoinHandle,
};

use crate::{
    emulator::{EmulatorReader, EmulatorWriter},
    intake::client::manager::ClientManagerHandle,
    pipeline::GameAction,
};

pub struct EmulatorClient {
    tasks: Vec<JoinHandle<()>>,
    client_manager: ClientManagerHandle,
    num_clients: usize,
    rom_path: String,
}

impl EmulatorClient {
    pub fn new(num_clients: usize, client_manager: ClientManagerHandle, rom_path: String) -> Self {
        Self {
            tasks: vec![],
            client_manager,
            num_clients,
            rom_path,
        }
    }

    pub fn start(&mut self) {
        for _ in 0..self.num_clients {
            let (frame_tx, frame_rx) = mpsc::channel::<DynamicImage>(10000);
            let (action_tx, mut action_rx) = mpsc::channel::<GameAction>(100);
            let client_manager_clone = self.client_manager.clone();
            let rom_path = self.rom_path.clone();
            self.tasks.push(tokio::spawn(async move {
                match client_manager_clone
                    .add_client(
                        Box::new(EmulatorReader::new(frame_rx)),
                        Box::new(EmulatorWriter::new(action_tx)),
                    )
                    .await
                {
                    Ok(id) => {
                        let emulator_task = tokio::task::spawn_blocking(move || {
                            tracing::info!("Emulator client starting game, with unique id: {}", id);
                            let mut desmume = desmume_rs::DeSmuME::init().unwrap();
                            if let Err(e) = desmume.open(&rom_path, true) {
                                let err_msg = format!("Failed to open ROM at path '{}': {:?}. Shutting down emulator task.", rom_path, e);
                                tracing::error!("{}", err_msg);
                                // Here, you could send this error back to the main app to be displayed in the UI
                                return;
                            };
                            desmume.volume_set(0);
                            tracing::info!("Emulator client opened game, with unique id: {}", id);
                            while desmume.is_running() {
                                if let Ok(action) = action_rx.try_recv() {
                                    // Map GameAction to keypad bitmask
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
                                desmume.cycle();

                                // Release all buttons between cycles
                                desmume.input_mut().keypad_update(0);
                                desmume.cycle();

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

                                match frame_tx.try_send(image) {
                                    Ok(_) => {}
                                    Err(err) => match err {
                                        TrySendError::Full(_) => {
                                            // Drop frame to keep real-time
                                            tracing::warn!("Dropping frame: channel full");
                                        }
                                        TrySendError::Closed(_) => {
                                            tracing::warn!("Frame channel closed, stopping emulator loop");
                                            break;
                                        }
                                    },
                                }
                            }
                        });
                        emulator_task.await.unwrap();
                    }
                    Err(e) => {
                        eprintln!("Error adding client: {}", e);
                    }
                }
            }));
        }
    }
}
