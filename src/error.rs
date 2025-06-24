use std::{array::TryFromSliceError, string::FromUtf8Error};

use thiserror::Error;
use uuid::Uuid;

// The single, top-level application error type.
#[derive(Error, Debug)]
pub enum AppError {
    // Network Errors
    #[error("Failed to bind to port {1}: {0}")]
    Bind(#[source] std::io::Error, u16),
    #[error("Failed to accept connection: {0}")]
    Accept(#[source] std::io::Error),
    #[error("Server shutdown error: {0}")]
    ServerShutdown(String),
    #[error("The server is already started.")]
    AlreadyStarted,

    // Client Errors
    #[error("Error sending shutdown to client handle {0}")]
    ClientShutdown(Uuid),
    #[error("A client operation failed: {0}")]
    Client(String),

    // Frame-related errors are wrapped
    #[error("Frame error: {0}")]
    Frame(#[from] FrameError),

    // Pipeline/Service Errors
    #[error("Pipeline error: {0}")]
    Pipeline(String),
    #[error("Reinforcement learning service error: {0}")]
    RLService(String),
    #[error("Action service error: {0}")]
    ActionService(String),

    // Generic I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
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
}
