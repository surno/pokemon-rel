use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, Interest};
use tokio::net::TcpStream;
use tracing::debug;

use crate::{error::FrameError, network::Frame};

#[derive(Debug)]
enum ReadState {
    WaitingForLength,
    WaitingForFrame { expected_length: u32 },
}

#[derive(Debug)]

pub struct FrameReader {
    reader: BufReader<TcpStream>,
    buffer: Vec<u8>,
    state: ReadState,
}

impl FrameReader {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            reader: BufReader::new(stream),
            buffer: vec![],
            state: ReadState::WaitingForLength,
        }
    }

    pub async fn is_connected(&self) -> bool {
        self.reader
            .get_ref()
            .ready(Interest::READABLE)
            .await
            .is_ok()
    }

    pub async fn shutdown(&mut self) -> Result<(), FrameError> {
        self.reader.shutdown().await.map_err(FrameError::ReadError)
    }

    pub async fn read_frame(&mut self) -> Result<Frame, FrameError> {
        loop {
            match &self.state {
                ReadState::WaitingForLength => {
                    // [length][tag][data]
                    // [length] is 4 bytes
                    while self.buffer.len() < 4 {
                        let mut temp_buffer = [0u8; 1024];
                        let bytes_read = self
                            .reader
                            .read(&mut temp_buffer)
                            .await
                            .map_err(FrameError::ReadError)?;

                        if bytes_read == 0 {
                            return Err(FrameError::InvalidFrameLength(0)); // Connection is closed
                        }

                        self.buffer.extend_from_slice(&temp_buffer[..bytes_read]);
                    }

                    let length_bytes: [u8; 4] = self.buffer[..4]
                        .try_into()
                        .map_err(|_| FrameError::InvalidFrameLength(self.buffer.len()))?;

                    let frame_length = u32::from_le_bytes(length_bytes);

                    // Remove the length from the buffer, downstream doesn't need it anymore
                    self.buffer.drain(..4);

                    self.state = ReadState::WaitingForFrame {
                        expected_length: frame_length,
                    };
                }
                ReadState::WaitingForFrame { expected_length } => {
                    while self.buffer.len() < *expected_length as usize {
                        let mut temp_buffer = [0u8; 1024];
                        let bytes_read = self
                            .reader
                            .read(&mut temp_buffer)
                            .await
                            .map_err(FrameError::ReadError)?;

                        if bytes_read == 0 {
                            debug!("Unable to read, is the connection closed?");
                            return Err(FrameError::InvalidFrameLength(0)); // Connection is closed
                        }

                        self.buffer.extend_from_slice(&temp_buffer[..bytes_read]);
                    }

                    let frame_data = self.buffer[..*expected_length as usize].to_vec();

                    self.buffer.drain(..*expected_length as usize);

                    self.state = ReadState::WaitingForLength;

                    // comment
                    debug!("{}", format!("Got Frame {}", frame_data.len()));
                    // send random 12 bytes of data for action.
                    let action = [0u8; 12];
                    let action_result = self.reader.write_all(&action).await;
                    if action_result.is_err() {
                        debug!("Error sending action: {:?}", action_result.err());
                    }
                    return Frame::try_from(frame_data.as_slice());
                }
            }
        }
    }
}
