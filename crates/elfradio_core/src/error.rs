use thiserror::Error;
use elfradio_types::AiError;
use elfradio_hardware::HardwareError;
use elfradio_types::PttSignalParseError;
use hound::Error as HoundError;
use serde_json::Error as SerdeJsonError;
use std::io;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Audio output channel closed")]
    AudioChannelClosed,
    #[error("PTT operation failed: {0}")]
    PttError(#[from] HardwareError),
    #[error("AI operation failed: {0}")]
    AiError(#[from] AiError),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Filesystem error: {0}")]
    IoError(#[from] io::Error),
    #[error("Audio playback error: {0}")]
    PlaybackError(String),
    #[error("Invalid PTT signal type: {0}")]
    PttSignalParseError(#[from] PttSignalParseError),
    #[error("Serial port not configured for PTT")]
    PttPortNotConfigured,
    #[error("Audio decoding error: {0}")]
    AudioDecodeError(#[from] HoundError),
    #[error("Serialization/deserialization error: {0}")]
    SerializationError(#[from] SerdeJsonError),
    #[error("Failed to send item to TX queue: {0}")]
    TxQueueSendError(String),
    #[error("Other core error: {0}")]
    Other(String),
    #[error("Audio processing error: {0}")]
    AudioError(String),
    #[error("Hardware interaction error: {0}")]
    HardwareError(String),
    #[error("Channel send error: {0}")]
    ChannelSendError(String),
    #[error("Channel receive error")]
    ChannelReceiveError,
    #[error("Invalid state for operation: {0}")]
    InvalidState(String),
    #[error("Task join error: {0}")]
    TaskJoinError(#[from] tokio::task::JoinError),
    #[error("A task is already running.")]
    TaskAlreadyRunning,
    #[error("No task is currently running.")]
    NoTaskRunning,
    #[error("An unknown error occurred: {0}")]
    Unknown(String),
    #[error("AI Request Failed: {0}")]
    AiRequestFailed(String),
    #[error("Database Error: {0}")]
    DatabaseError(#[from] elfradio_db::DbError),
    #[error("Task with ID {0} not found")]
    TaskNotFound(Uuid),
    #[error("Task Error: {0}")]
    TaskError(String),
    #[error("AI provider is not configured. Please configure in settings.")]
    AiNotConfigured,
} 