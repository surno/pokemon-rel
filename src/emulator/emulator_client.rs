use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    io::Write,
    sync::Arc,
};

use desmume_rs::input::{Key, keymask};
use image::{DynamicImage, RgbImage, RgbaImage};
use imghash::ImageHasher;
use tokio::{
    io::BufWriter,
    sync::{broadcast, mpsc},
    task::JoinHandle,
    time::{Duration, Instant, sleep},
};
use tracing::Instrument;

use crate::{
    Frame,
    emulator::{EmulatorReader, EmulatorWriter},
    intake::{client::manager::ClientManagerHandle, frame::writer::FramedAsyncBufferedWriter},
    pipeline::GameAction,
};

pub struct EmulatorClient {
    tasks: Vec<JoinHandle<()>>,
    client_manager: ClientManagerHandle,
    num_clients: usize,
}

impl EmulatorClient {
    pub fn new(num_clients: usize, client_manager: ClientManagerHandle) -> Self {
        Self {
            tasks: vec![],
            client_manager,
            num_clients,
        }
    }

    pub fn start(&mut self) {
        for _ in 0..self.num_clients {
            let (frame_tx, frame_rx) = mpsc::channel::<DynamicImage>(10000);
            let (action_tx, action_rx) = mpsc::channel::<Frame>(100);
            let mut client_manager_clone = self.client_manager.clone();
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
                            let result = desmume.open(
                                "/Users/tony/Projects/pokemon-shiny/POKEMON_B_IRBO01_00.nds",
                                true,
                            );
                            desmume.volume_set(0);
                            tracing::info!("Emulator client opened game, with unique id: {}", id);
                            while desmume.is_running() {
                                // Press the start button
                                if desmume.get_ticks() % 1000 == 0 {
                                    tracing::info!("Pressing down");
                                    desmume.input_mut().keypad_update(keymask(Key::Down));
                                } else {
                                    desmume.input_mut().keypad_update(keymask(Key::Start));
                                }
                                desmume.cycle();

                                // Release the start button
                                desmume.input_mut().keypad_update(0);
                                desmume.cycle();

                                let buffer = desmume.display_buffer_as_rgbx();

                                // -- pixel order is B G R A; convert to R G B
                                let mut new_buffer = Vec::new();
                                for i in (0..buffer.len()).step_by(4) {
                                    let b = buffer[i];
                                    let g = buffer[i + 1];
                                    let r = buffer[i + 2];
                                    // let a = buffer[i + 3];
                                    new_buffer.push(r);
                                    new_buffer.push(g);
                                    new_buffer.push(b);
                                }

                                let image = DynamicImage::ImageRgb8(
                                    RgbImage::from_raw(
                                        desmume_rs::SCREEN_WIDTH as u32,
                                        desmume_rs::SCREEN_HEIGHT_BOTH as u32,
                                        new_buffer,
                                    )
                                    .unwrap(),
                                );

                                match frame_tx.blocking_send(image) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        tracing::error!("Error sending frame: {}", e);
                                        break;
                                    }
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
