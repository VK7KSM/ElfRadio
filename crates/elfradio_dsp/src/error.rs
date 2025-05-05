use thiserror::Error;
use image::ImageError;
use std::io;

#[derive(Error, Debug)]
pub enum VadError {
    #[error("Unsupported sample rate: {0}. Supported rates: 8000, 16000, 32000, 48000")]
    UnsupportedSampleRate(u32),
    #[error("Unsupported frame size: {0} ms. Supported sizes: 10, 20, 30 ms")]
    UnsupportedFrameSize(usize),
    #[error("WebRTC VAD error: {0}")]
    VADInternalError(String),
    #[error("Input audio chunk size mismatch. Expected {expected}, got {actual}")]
    ChunkSizeMismatch { expected: usize, actual: usize },
}

#[derive(Error, Debug)]
pub enum DspError {
    #[error("VAD processing error: {0}")]
    VadError(#[from] VadError),

    #[error("Failed to load or decode image: {0}")]
    ImageError(#[from] ImageError),

    #[error("SSTV encoding failed: {0}")]
    SstvEncodeError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("Unsupported image dimensions for SSTV mode: expected {expected_w}x{expected_h}, got {actual_w}x{actual_h}")]
    UnsupportedDimensions {
        expected_w: u32,
        expected_h: u32,
        actual_w: u32,
        actual_h: u32,
    },

    // --- CW Errors ---
    #[error("Invalid WPM value: {0}. Must be greater than 0.")]
    InvalidWpm(u32),
    #[error("Unsupported character for CW encoding: '{0}'")]
    UnsupportedCharacter(char),
}
