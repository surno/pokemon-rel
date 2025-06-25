use image::{DynamicImage, RgbImage, RgbaImage};
use tokio::{
    io::BufWriter,
    sync::{broadcast, mpsc},
    task::JoinHandle,
};

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
            let (frame_tx, frame_rx) = broadcast::channel::<DynamicImage>(100);
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
                    Ok(_) => {
                        let mut desmume = desmume_rs::DeSmuME::init().unwrap();
                        let result = desmume.open(
                            "/Users/tony/Projects/pokemon-shiny/POKEMON_B_IRBO01_00.nds",
                            true,
                        );
                        while desmume.is_running() {
                            desmume.cycle();
                            let buffer = desmume.display_buffer_as_rgbx();
                            let image = DynamicImage::ImageRgba8(
                                RgbaImage::from_raw(
                                    desmume_rs::SCREEN_WIDTH as u32,
                                    desmume_rs::SCREEN_HEIGHT_BOTH as u32,
                                    buffer,
                                )
                                .unwrap(),
                            );
                            frame_tx.send(image).unwrap();
                        }
                    }
                    Err(e) => {
                        eprintln!("Error adding client: {}", e);
                    }
                }
            }));
        }
    }
}
