use std::{array::TryFromSliceError, string::FromUtf8Error};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Service error: {0}")]
    Service(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("Client error: {0}")]
    Client(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Emulator error: {0}")]
    Emulator(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("UI error: {0}")]
    Ui(String),
    #[error("Unknown error")]
    Unknown,
}

// FrameError remains a detailed, specific error type for frame parsing.
#[derive(Error, Debug)]
pub enum FrameError {
    #[error("Failed to read frame: {0}")]
    Read(std::io::Error),
    #[error("Invalid frame length, expected {0} bytes, got {1}")]
    InvalidFrameLength(usize, usize),
    #[error("Invalid frame tag, got {0}")]
    InvalidFrameTag(u8),
    #[error("Invalid program from slice: {0}")]
    InvalidProgram(TryFromSliceError),
    #[error("Invalid version from slice: {0}")]
    InvalidVersion(TryFromSliceError),
    #[error("Invalid name length from slice: {0}")]
    InvalidNameLength(TryFromSliceError),
    #[error("Invalid name from utf8: {0}")]
    InvalidName(FromUtf8Error),
    #[error("Invalid width from slice: {0}")]
    InvalidWidth(TryFromSliceError),
    #[error("Invalid height from slice: {0}")]
    InvalidHeight(TryFromSliceError),
    #[error("Invalid pixels length, got {0}x{1} = {2}, expected {3}")]
    InvalidPixelsLength(u32, u32, usize, usize),
    #[error("Failed to convert slice to frame: {0}")]
    TryFromSlice(TryFromSliceError),
    #[error("Failed to send frame: {0}")]
    Send(String),
}
