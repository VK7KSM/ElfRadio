use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    InputCallbackInfo,
    SampleFormat,
    StreamError,
    SupportedStreamConfig,
};
use elfradio_types::{AudioMessage, PttSignal};
// use serialport::{SerialPortInfo, SerialPortType}; // <-- 注释掉或删除这行
use std::{io, time::Duration};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info, trace, warn};

// --- 新增: 音频输出发送器类型别名 ---
pub type AudioOutputSender = mpsc::UnboundedSender<Vec<f32>>;

#[derive(Error, Debug)]
pub enum HardwareError {
    // --- 通用错误 ---
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    // --- 设备错误 ---
    #[error("Audio host unavailable: {0}")]
    HostError(#[from] cpal::HostUnavailable),

    #[error("Failed to access audio devices: {0}")]
    DevicesError(#[from] cpal::DevicesError),

    #[error("Failed to get device name: {0}")]
    DeviceNameError(#[from] cpal::DeviceNameError),

    #[error("Audio device not found: {0}")]
    DeviceNotFound(String),

    #[error("No default audio device available: {0}")]
    DefaultDeviceError(String),

    // --- 流配置/处理错误 ---
    #[error("Failed to get stream configuration: {0}")]
    StreamConfigError(#[from] cpal::DefaultStreamConfigError),

    #[error("Failed to build audio stream: {0}")]
    BuildStreamError(#[from] cpal::BuildStreamError),

    #[error("Failed to play audio stream: {0}")]
    PlayStreamError(#[from] cpal::PlayStreamError),

    #[error("Failed to pause audio stream: {0}")]
    PauseStreamError(#[from] cpal::PauseStreamError),

    #[error("Audio stream error: {0}")]
    StreamError(#[from] cpal::StreamError),

    #[error("Unsupported audio sample format")]
    UnsupportedSampleFormat,

    // --- 串口错误 ---
    #[error("Serial port error: {0}")]
    SerialPortError(#[from] serialport::Error),

    #[error("Serial port not found: {0}")]
    SerialPortNotFound(String),

    #[error("PTT operation error: {0}")]
    PttError(String),

    // --- 其他错误 ---
    #[error("{0}")]
    GenericError(String),
}

/// Lists available audio input and output devices.
pub fn list_audio_devices() -> Result<Vec<String>, HardwareError> {
    debug!("Listing audio devices...");
    let host = cpal::default_host();
    let mut devices = Vec::new();

    let output_devices = host.output_devices()?;
    for device in output_devices {
        let name = device.name()?;
        trace!("Found output device: {}", name);
        devices.push(name);
    }

    let input_devices = host.input_devices()?;
    for device in input_devices {
        let name = device.name()?;
        trace!("Found input device: {}", name);
        devices.push(name);
    }

    // Deduplicate the list
    devices.sort_unstable();
    devices.dedup();

    debug!("Found {} unique audio devices.", devices.len());
    Ok(devices)
}

/// Lists available serial ports suitable for PTT (USB serial).
pub fn list_serial_ports() -> Result<Vec<String>, HardwareError> {
    debug!("Listing available serial ports...");
    let ports = serialport::available_ports()?;
    let mut port_names = Vec::new();

    for p in ports {
        trace!("Found port: {}, type: {:?}", p.port_name, p.port_type);
        // Often filter for USB ports for PTT/CAT, but list all for now.
        // match p.port_type {
        //     SerialPortType::UsbPort(_) => {
        //         port_names.push(p.port_name);
        //     }
        //     _ => {} // Ignore others like Bluetooth, Unknown
        // }
        port_names.push(p.port_name); // List all for now
    }
    debug!("Found {} serial ports.", port_names.len());
    Ok(port_names)
}

/// Provides control over a running audio stream.
/// Currently holds the stream object itself. Dropping this struct stops the stream.
pub struct StreamControl {
    stream: cpal::Stream,
}

impl StreamControl {
    /// Explicitly pause the audio stream.
    pub fn pause(&self) -> Result<(), HardwareError> {
        self.stream.pause().map_err(HardwareError::PauseStreamError)
    }

    /// Explicitly resume the audio stream.
    pub fn play(&self) -> Result<(), HardwareError> {
         self.stream.play().map_err(HardwareError::PlayStreamError)
    }

    // Note: The stream is automatically stopped when StreamControl is dropped.
}

/// Calculates the Root Mean Square (RMS) of a slice of f32 audio samples.
fn calculate_rms(data: &[f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = data.iter().map(|&sample| sample * sample).sum();
    let mean_sq = sum_sq / (data.len() as f32);
    mean_sq.sqrt()
}

/// Sets the PTT (Push-To-Talk) state using a serial port signal.
///
/// # Arguments
/// * `port_name` - The name of the serial port (e.g., "COM3", "/dev/ttyUSB0").
/// * `signal` - Which signal line to use (`Rts` or `Dtr`).
/// * `state` - `true` to activate PTT (ON), `false` to deactivate (OFF).
/// * `pre_delay_ms` - Delay (ms) *after* activating signal *before* returning (for PTT ON).
/// * `post_delay_ms` - Delay (ms) *before* deactivating signal *after* call (for PTT OFF).
pub async fn set_ptt(
    port_name: &str,
    signal: PttSignal,
    state: bool,
    pre_delay_ms: u64,
    post_delay_ms: u64,
) -> Result<(), HardwareError> {
    trace!(port = port_name, ?signal, ptt_state = state, pre_delay_ms, post_delay_ms, "Setting PTT");

    // Find the correct port info first to ensure it exists before trying to open
    let ports = serialport::available_ports()?;
    let port_info = ports.into_iter().find(|p| p.port_name == port_name);

    if port_info.is_none() {
         error!("Specified PTT port '{}' not found in available ports.", port_name);
         return Err(HardwareError::SerialPortNotFound(port_name.to_string()));
    }

    // Open the serial port
    // Use a short timeout as we only control signals, not data transfer.
    let mut port = serialport::new(port_name, 9600) // Baud rate likely irrelevant for signal control
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|e| {
             error!("Failed to open serial port '{}': {}", port_name, e);
             HardwareError::SerialPortError(e)
         })?;

    if state { // Activate PTT (ON)
        debug!(port = port_name, ?signal, "Activating PTT");
        match signal {
            PttSignal::Rts => port.write_request_to_send(true)?,
            PttSignal::Dtr => port.write_data_terminal_ready(true)?,
        }
        // Wait for pre-delay *after* activating signal
        if pre_delay_ms > 0 {
            trace!("Waiting for pre-delay: {}ms", pre_delay_ms);
            sleep(Duration::from_millis(pre_delay_ms)).await;
        }
        trace!("PTT Activated and pre-delay finished");
    } else { // Deactivate PTT (OFF)
        // Wait for post-delay *before* deactivating signal
        if post_delay_ms > 0 {
            trace!("Waiting for post-delay: {}ms", post_delay_ms);
            sleep(Duration::from_millis(post_delay_ms)).await;
        }
        debug!(port = port_name, ?signal, "Deactivating PTT");
        match signal {
            PttSignal::Rts => port.write_request_to_send(false)?,
            PttSignal::Dtr => port.write_data_terminal_ready(false)?,
        }
         trace!("PTT Deactivated after post-delay");
    }

    // Port is closed automatically when `port` goes out of scope here.
    Ok(())
}

/// Starts capturing audio from the specified input device.
///
/// Sends audio data chunks and RMS values over the provided MPSC channel.
///
/// # Arguments
/// * `device_name` - Optional name of the specific input device to use. If `None`, the default input device is used.
/// * `config` - The desired stream configuration (sample rate, channels, format) obtained from `device.supported_input_configs()`.
/// * `data_tx` - An unbounded MPSC sender to send `AudioMessage`s (Data chunks, RMS) to.
///
/// # Returns
/// A `StreamControl` struct which, when dropped, will stop the audio stream.
pub fn start_audio_input_stream(
    device_name: Option<&str>,
    config: &SupportedStreamConfig,
    data_tx: mpsc::UnboundedSender<AudioMessage>,
) -> Result<StreamControl, HardwareError> {
    debug!(?device_name, ?config, "Starting audio input stream...");

    // Ensure requested format is F32, as callback expects it
    if config.sample_format() != SampleFormat::F32 {
        error!("Unsupported sample format requested: {:?}. Only F32 is handled.", config.sample_format());
        return Err(HardwareError::UnsupportedSampleFormat);
    }

    let host = cpal::default_host();

    // Find the device
    let device = match device_name {
        Some(name) => {
            trace!("Searching for specific input device: {}", name);
            host.input_devices()?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .ok_or_else(|| HardwareError::DeviceNotFound(name.to_string()))?
        }
        None => {
            trace!("Using default input device.");
            host.default_input_device()
                .ok_or_else(|| HardwareError::DeviceNotFound("Default input device".to_string()))?
        }
    };
    info!("Using audio input device: {}", device.name()?);

    // --- Build Input Stream ---
    // Define the error callback
    let err_fn = {
        let data_tx = data_tx.clone(); // Clone sender for error callback
        move |err: StreamError| {
            error!("An error occurred on audio input stream: {}", err);
            // Send error message over the channel
            let _ = data_tx.send(AudioMessage::Error(err.to_string()));
        }
    };

    // Define the data callback
    let data_callback = move |data: &[f32], _: &InputCallbackInfo| {
        trace!("Received audio data chunk, len: {}", data.len());
        // 1. Calculate RMS
        let rms = calculate_rms(data);
        // 2. Send RMS (ignore error if receiver dropped)
        let _ = data_tx.send(AudioMessage::Rms(rms));
        // 3. Send Data Copy (ignore error if receiver dropped)
        //    Use to_vec() to create an owned Vec<f32> from the slice
        let _ = data_tx.send(AudioMessage::Data(data.to_vec()));
    };

    // Build the stream
    debug!("Building input stream with config: {:?}", config);
    let stream = device.build_input_stream(
        &config.config(),
        data_callback,
        err_fn,
        None, // Optional timeout
    )?;
    debug!("Audio input stream built successfully.");

    // --- Play Stream ---
    stream.play()?;
    info!("Audio input stream started successfully.");

    // --- Return Control ---
    Ok(StreamControl { stream })
}

// --- 新增: 启动音频输出流 ---
/// Starts the audio output stream using the specified device and configuration.
///
/// Returns a `StreamControl` handle and an `AudioOutputSender` channel
/// to send audio data (`Vec<f32>`) to the stream.
pub fn start_audio_output_stream(
    device_name: Option<&str>,
    config: &cpal::SupportedStreamConfig,
) -> Result<(StreamControl, AudioOutputSender), HardwareError> {
    info!(
        "Attempting to start audio output stream with device: {:?}, config: {:?}",
        device_name,
        config.config() // Log the actual StreamConfig part
    );

    let host = cpal::default_host();

    // Find the output device
    let device = match device_name {
        Some(name) => host
            .output_devices()?
            .find(|d| d.name().map(|n| n == name).unwrap_or(false))
            .ok_or_else(|| HardwareError::DeviceNotFound(name.to_string()))?,
        None => host
            .default_output_device()
            .ok_or(HardwareError::DefaultDeviceError("No default output device available".to_string()))?,
    };

    info!("Using output device: {}", device.name()?);

    // --- Create channel for sending audio data to the callback ---
    let (data_tx, mut data_rx): (
        AudioOutputSender,
        mpsc::UnboundedReceiver<Vec<f32>>,
    ) = mpsc::unbounded_channel();

    // --- Define the audio output callback ---
    let output_callback = {
        let mut current_chunk: Vec<f32> = Vec::new(); // Buffer for the chunk being played
        let mut chunk_pos: usize = 0; // Current read position within current_chunk

        move |data: &mut cpal::Data, _: &cpal::OutputCallbackInfo| {
            if let Some(output) = data.as_slice_mut::<f32>() {
                let mut output_pos = 0; // Position within the cpal `output` buffer

                while output_pos < output.len() {
                    // Check if we need more data from our internal chunk buffer
                    if chunk_pos >= current_chunk.len() {
                        // Try to receive a new chunk *non-blockingly*
                        match data_rx.try_recv() {
                            Ok(new_chunk) => {
                                if !new_chunk.is_empty() {
                                    // Received new data
                                    current_chunk = new_chunk;
                                    chunk_pos = 0;
                                } else {
                                    // Received an empty chunk, treat as silence trigger
                                    warn!("Received empty audio chunk for output.");
                                    current_chunk.clear();
                                    chunk_pos = 0;
                                }
                            }
                            Err(mpsc::error::TryRecvError::Empty) => {
                                // No data currently available from the channel
                                 warn!("Audio output buffer underrun - channel empty.");
                                current_chunk.clear(); // Ensure silence generation
                                chunk_pos = 0;
                            }
                            Err(mpsc::error::TryRecvError::Disconnected) => {
                                // Channel closed, log and prepare to write silence
                                error!("Audio output channel disconnected!");
                                current_chunk.clear(); // Ensure silence generation
                                chunk_pos = 0;
                                // No break here, let the loop write silence
                            }
                        }
                    }

                    // Write samples from current_chunk or silence
                    let available_in_chunk = current_chunk.len().saturating_sub(chunk_pos);
                    let needed_for_output = output.len() - output_pos;
                    let samples_to_write = available_in_chunk.min(needed_for_output);

                    if samples_to_write > 0 {
                        // Write data from the chunk
                        let chunk_slice = &current_chunk[chunk_pos..chunk_pos + samples_to_write];
                        output[output_pos..output_pos + samples_to_write].copy_from_slice(chunk_slice);
                        chunk_pos += samples_to_write;
                        output_pos += samples_to_write;
                    } else {
                        // No data in current_chunk (either empty or exhausted after try_recv)
                        // Write silence for the remaining needed output samples
                        let samples_to_silence = needed_for_output;
                        if samples_to_silence > 0 {
                             // Only log underrun when actually writing silence
                            // warn!("Audio output buffer underrun - writing silence for {} samples.", samples_to_silence);
                            output[output_pos..output_pos + samples_to_silence].fill(0.0);
                        }
                        // Exit the while loop as output is now full (with silence if necessary)
                         break;
                    }
                }
            } else {
                 error!("Output data format is not F32 as expected!");
                 // Optionally fill the raw buffer with zeros if possible/needed
                 // data.fill(0); // 不安全，需要知道具体类型和大小
            }
        }
    };

    // --- Define the error callback ---
    let err_fn = |err: StreamError| {
        error!("An error occurred on audio output stream: {}", err);
        // We don't have a channel back here easily, might need one if critical
    };

    // --- Build the output stream ---
    // We use build_output_stream_raw to explicitly request F32 format for the callback buffer.
    // CPAL will handle the conversion from f32 to the device's native format.
    let stream = device
        .build_output_stream_raw(
            &config.config(), // <-- 传递引用
            SampleFormat::F32,
            output_callback,
            err_fn,
            None,
        )?; // <-- 这个 ? 应该可以工作了

    // Play the stream
    stream.play()?; // Uses PlayStreamError via ? and From trait

    info!("Audio output stream started successfully.");

    Ok((StreamControl { stream }, data_tx))
}

// Potentially add functions for audio stream handling later
// pub async fn start_audio_input_stream(...) -> Result<...>
// pub async fn start_audio_output_stream(...) -> Result<...>
