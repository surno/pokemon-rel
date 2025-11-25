use chrono::Utc;
use image::{DynamicImage, RgbImage};
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[cfg(target_os = "macos")]
use objc2::rc::autoreleasepool;

use crate::common::Frame;
use crate::{common::game_action::GameAction, error::AppError};

/// Initialize DeSmuME emulator on the main thread (required for Metal thread safety).
/// This function handles all Metal initialization before the emulator is moved to a background thread.
pub fn initialize_emulator(
    rom_path: String,
    renderer: String,
) -> Result<desmume_rs::DeSmuME, AppError> {
    // Determine if Metal renderer is requested
    // We'll use two-phase initialization: start with software, then switch to Metal
    let wants_metal = match renderer.as_str() {
        "metal" => {
            #[cfg(target_os = "macos")]
            {
                true
            }
            #[cfg(not(target_os = "macos"))]
            {
                tracing::warn!("Metal renderer requested but not available on this platform, using software rasterizer");
                false
            }
        }
        "software" => false,
        "auto" | _ => {
            // Auto: Use Metal on macOS, software on other platforms
            #[cfg(target_os = "macos")]
            {
                true
            }
            #[cfg(not(target_os = "macos"))]
            {
                false
            }
        }
    };

    // Always start with software rasterizer to set up NDS_Init and GPU
    // This ensures GPU subsystem exists before we try to switch to Metal
    let initial_renderer = desmume_rs::Renderer3D::SoftRasterizer;

    let options = desmume_rs::InitOptions {
        audio_core: desmume_rs::AudioCore::Dummy,
        audio_buffer_size: None,
        renderer_3d: initial_renderer,
        init_sdl_timer: false,
    };

    tracing::info!("Initializing emulator with software rasterizer (GPU setup phase)");
    let mut desmume = desmume_rs::DeSmuME::init_with_options(options)
        .map_err(|e| AppError::Emulator(e.to_string()))?;

    // Now switch to Metal if requested (GPU exists, safe to switch)
    // This happens on the main thread where Metal initialization is safe
    #[cfg(target_os = "macos")]
    {
        if wants_metal {
            tracing::info!("Switching to Metal renderer (bootstrap and renderer switch phase)");
            match desmume.init_metal() {
                Ok(_) => {
                    tracing::info!("Successfully switched to Metal renderer");
                }
                Err(e) => {
                    tracing::warn!("Failed to switch to Metal renderer: {}, continuing with software rasterizer", e);
                }
            }
        }
    }

    // Verify Metal renderer is active on macOS
    #[cfg(target_os = "macos")]
    {
        if desmume.has_metal() {
            tracing::info!("Metal renderer is active");
        } else {
            tracing::info!("Using software rasterizer");
        }
    }

    if let Err(e) = desmume.open(&rom_path, true) {
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

pub struct EmulatorClient {
    cancel_token: CancellationToken,
    emulator_thread: Option<std::thread::JoinHandle<()>>,
}

impl EmulatorClient {
    /// Create a new EmulatorClient with a pre-initialized DeSmuME instance.
    /// The DeSmuME must be initialized on the main thread (via `initialize_emulator`)
    /// to ensure Metal thread safety.
    pub fn new(
        action_rx: Receiver<GameAction>,
        frame_tx: Sender<Frame>,
        desmume: desmume_rs::DeSmuME,
    ) -> Self {
        let cancel_token = CancellationToken::new();
        let mut emulator = Emulator::new(action_rx, frame_tx, desmume);
        Self {
            cancel_token: cancel_token.clone(),
            emulator_thread: Some(std::thread::spawn(move || {
                emulator.run(cancel_token.clone())
            })),
        }
    }

    pub fn stop(&mut self) {
        self.cancel_token.cancel();
        if let Some(thread) = self.emulator_thread.take() {
            thread.join().expect("Emulator thread panicked");
        }
    }
}

impl Drop for EmulatorClient {
    fn drop(&mut self) {
        self.stop();
    }
}

struct Emulator {
    action_rx: Receiver<GameAction>,
    frame_tx: Sender<Frame>,
    desmume: desmume_rs::DeSmuME,
    id: Uuid,
}

impl Emulator {
    pub fn new(
        action_rx: Receiver<GameAction>,
        frame_tx: Sender<Frame>,
        desmume: desmume_rs::DeSmuME,
    ) -> Self {
        Self {
            action_rx,
            frame_tx,
            desmume,
            id: Uuid::new_v4(),
        }
    }

    fn release_key(&mut self) {
        self.desmume.input_mut().keypad_update(0);
    }

    fn prepare_action(&mut self, action: GameAction) {
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
        };
        if mask != 0 {
            self.desmume.input_mut().keypad_update(mask);
            tracing::info!("Applied keypad mask {:#018b} for action {:?}", mask, action);
        } else {
            tracing::warn!("No keypad mapping for action {:?}", action);
        }
    }

    fn get_dynamic_image(&mut self) -> Option<DynamicImage> {
        let buffer = self.desmume.display_buffer_as_rgbx();
        let mut new_buffer: Vec<u8> = Vec::with_capacity(buffer.len() / 4 * 3);
        // -- pixel order is B G R A; convert to R G B
        for chunk in buffer.chunks_exact(4) {
            // chunk = [B, G, R, A]
            new_buffer.extend_from_slice(&[chunk[2], chunk[1], chunk[0]]);
        }
        let rgb_image = RgbImage::from_raw(
            desmume_rs::SCREEN_WIDTH as u32,
            desmume_rs::SCREEN_HEIGHT_BOTH as u32,
            new_buffer,
        );
        match rgb_image {
            Some(rgb_image) => {
                let image = DynamicImage::ImageRgb8(rgb_image);
                Some(image)
            }
            None => {
                tracing::error!("Failed to convert buffer to RGB image");
                None
            }
        }
    }

    fn process_frame(&mut self) {
        let image = self.get_dynamic_image();
        match image {
            Some(image) => {
                match self
                    .frame_tx
                    .try_send(Frame::new(self.id, image, Utc::now(), Uuid::new_v4()))
                {
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
            None => {
                tracing::error!("Failed to get dynamic image");
            }
        }
    }

    pub fn run(&mut self, cancel_token: CancellationToken) {
        tracing::info!("Emulator starting game, with unique id: {}", self.id);

        // On macOS, Metal requires an autorelease pool on the thread
        // Create a pool that lives for the entire thread lifetime
        #[cfg(target_os = "macos")]
        {
            autoreleasepool(|_pool| {
                self.run_inner(cancel_token);
            });
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            self.run_inner(cancel_token);
        }
    }
    
    fn run_inner(&mut self, cancel_token: CancellationToken) {
        // DeSmuME is already initialized on the main thread
        while self.desmume.is_running() && !cancel_token.is_cancelled() {
            match self.action_rx.try_recv() {
                Ok(action) => {
                    self.prepare_action(action);
                }
                Err(TryRecvError::Disconnected) => {
                    tracing::error!("Action channel closed, stopping emulator loop");
                    break;
                }
                Err(_) => {
                    // No action to process, cycle the emulator and process the frame
                }
            }
            self.desmume.cycle();
            self.release_key();
            self.process_frame();
        }
        tracing::info!("Emulator stopped game, with unique id: {}", self.id);
    }
}
