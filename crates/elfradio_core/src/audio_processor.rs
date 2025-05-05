// Audio Input Processing Logic

use super::error::CoreError; // Use the new error module from the parent
use super::state::AppState; // Use the state module from the parent
use elfradio_types::{
    AudioMessage, LogContentType, LogDirection, LogEntry, AiConfig, // Added AiConfig
};
// use elfradio_ai::{AiClient, SttParams}; // Add if STT logic is included later
use elfradio_dsp::vad::VadProcessor;
use webrtc_vad::VadMode;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::watch; // 新增导入
use std::path::Path;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use chrono::Utc;
use serde_json;
use tracing::{debug, error, info, warn, trace, instrument};
use hound; // Import the hound crate directly
use elfradio_ai::SttParams; // Added import for STT logic
use elfradio_db::insert_log_entry; // 导入数据库插入函数
use uuid::Uuid; // 用于 task_id
// use elfradio_ai::AiError; // 删除或注释掉此行

// Define a specific Result type alias for this module if needed, or use the parent's
#[allow(dead_code)]
type AudioProcessingOutcome<T> = std::result::Result<T, CoreError>;

// 处理传入音频 (Moved from processing.rs)
#[instrument(skip(_app_state, _audio_rx))]
pub async fn process_incoming_audio(
    _app_state: Arc<AppState>,
    _audio_rx: mpsc::UnboundedReceiver<AudioMessage>,
) {
    info!("Starting incoming audio processing task.");

    // VAD 处理设置
    let sample_rate = 16000;
    let frame_size = (sample_rate / 100) as usize;

    let _vad_processor = match VadProcessor::new(sample_rate, frame_size, VadMode::Aggressive) {
        Ok(vp) => vp,
        Err(e) => {
            error!("Failed to initialize VAD Processor: {}", e);
            return;
        }
    };

    trace!("VAD initialized with frame size: {}", frame_size);
    // 这里是 VAD 处理逻辑的剩余部分
    // TODO: Implement VAD detection loop, segment saving, and STT request generation
    // loop {
    //    if let Some(audio_msg) = _audio_rx.recv().await {
    //        match audio_msg {
    //             AudioMessage::Data(data) => {
    //                 // Process data with _vad_processor
    //                 // If speech detected and segment complete:
    //                 // let segment_index = ...; // manage index
    //                 // let task_dir = ...; // determine task directory
    //                 // if let Err(e) = save_audio_segment(_app_state.clone(), &data, &task_dir, segment_index).await {
    //                 //     error!("Failed to save audio segment: {}", e);
    //                 // }
    //                 // Potentially call process_stt_request here or queue it
    //             }
    //             AudioMessage::EndOfStream => {
    //                 info!("End of incoming audio stream.");
    //                 break;
    //             }
    //        }
    //    } else {
    //         info!("Audio receiver channel closed.");
    //         break;
    //    }
    // }
}

// 保存元数据 (Moved from processing.rs)
#[instrument(skip(_app_state, _data))]
async fn save_metadata(
    _app_state: Arc<AppState>,
    _data: &serde_json::Value,
    task_dir: &Path,
    filename: &str,
) -> AudioProcessingOutcome<()> {
    let full_path = task_dir.join(filename);
    info!("Saving metadata to: {:?}", full_path);

    let json_string = serde_json::to_string_pretty(_data)?;
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&full_path)?;

    let mut writer = BufWriter::new(file);
    writeln!(writer, "{}", json_string)?;

    Ok(())
}

// 保存音频片段 (Moved from processing.rs)
#[instrument(skip(_app_state, _audio_data))]
async fn save_audio_segment(
    _app_state: Arc<AppState>,
    _audio_data: &[f32],
    task_dir: &Path,
    segment_index: u32,
) -> AudioProcessingOutcome<()> {
    let filename = format!("segment_{}.wav", segment_index);
    let full_path = task_dir.join(filename);
    info!("Saving audio segment to: {:?}", full_path);

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int
    };

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&full_path)?;

    let writer = BufWriter::new(file);
    let mut wav_writer = hound::WavWriter::new(writer, spec)?;

    for &sample in _audio_data {
        let sample_i16 = (sample * i16::MAX as f32) as i16;
        wav_writer.write_sample(sample_i16)?;
    }

    wav_writer.finalize()?;

    let _log_entry = LogEntry {
        timestamp: Utc::now(),
        direction: LogDirection::Internal,
        content_type: LogContentType::Audio,
        content: full_path.to_string_lossy().into_owned(),
    };

    Ok(())
}

// ----------------------------------------------------------------------------
// STT Processing Logic
// ----------------------------------------------------------------------------

/// Constructs STT parameters based on the application configuration.
fn construct_stt_params(ai_config: &AiConfig) -> Result<SttParams, CoreError> {
    let language_code = ai_config.google.as_ref()
        .and_then(|g| g.stt_language.clone())
        .or_else(|| {
             warn!("STT language code not found in Google config, using default 'en-US'.");
             Some("en-US".to_string())
        })
        .unwrap();

    let sample_rate_value = 16000;
    debug!("Using language '{}' and sample rate {} for STT", language_code, sample_rate_value);

    Ok(SttParams {
        language_code,
        sample_rate: sample_rate_value,
        model: None,
        audio_format: "LINEAR16".to_string(),
    })
}

/// Processes an audio chunk using the configured AI service for Speech-to-Text.
///
/// Expects `audio_bytes` to contain raw audio data in a format compatible
/// with the STT service (typically WAV or raw PCM, check AiClient implementation).
#[instrument(skip(app_state, audio_data), fields(audio_len = audio_data.len()))]
pub async fn process_stt_request(
    app_state: Arc<AppState>,
    audio_data: Vec<u8>,
) -> Result<String, CoreError> {
    info!("Processing STT request for audio chunk of size: {}", audio_data.len());

    if audio_data.is_empty() {
        warn!("Received empty audio data for STT request, skipping.");
        // 返回空字符串表示没有识别结果
        return Ok("".to_string());
    }

    // 1. 从配置构造 STT 参数
    let stt_params = construct_stt_params(&app_state.config.ai_settings)?;
    debug!("Constructed STT Params: {:?}", stt_params);

    // 2. 获取 AI 客户端的读锁
    let ai_client_guard = app_state.ai_client.read().await;

    // 3. 检查 AI 客户端是否存在
    if let Some(client) = ai_client_guard.as_ref() {
        // 如果客户端存在，调用 STT
        match client.speech_to_text(&audio_data, &stt_params).await {
            Ok(text) => {
                info!("Successfully transcribed audio: {}", text);
                
                // 4. 添加数据库记录代码
                // 获取 task_id 和 db_pool
                let task_id_opt: Option<Uuid> = {
                    let task_guard = app_state.active_task.lock().await;
                    task_guard.as_ref().map(|info| info.id)
                };
                let db_pool = &app_state.db_pool;

                if let Some(task_id) = task_id_opt {
                    let entry = LogEntry {
                        timestamp: Utc::now(),
                        direction: LogDirection::Incoming,
                        content_type: LogContentType::Text,
                        content: text.clone(), // 使用转录的文本
                    };

                    // 调用数据库插入函数
                    if let Err(e) = insert_log_entry(db_pool, task_id, &entry).await {
                        error!(task_id = %task_id, "Failed to insert STT log entry into database: {:?}", e);
                        // 决定是否需要向上传播错误
                    } else {
                        debug!(task_id = %task_id, "STT log entry inserted into database.");
                    }
                } else {
                    warn!("No active task found when trying to log STT result to database.");
                }
                
                Ok(text)
            },
            Err(e) => {
                error!("STT Error: {:?}. Transcription not logged.", e);
                Err(CoreError::AiRequestFailed(format!("STT failed: {}", e)))
            }
        }
    } else {
        // 如果客户端为 None，使用新定义的 AiNotConfigured 错误
        warn!("Attempted to call STT, but AI provider is not configured.");
        Err(CoreError::AiNotConfigured) // 使用新定义的错误
    }
}

// TODO: Implement process_stt_request function if needed
// pub async fn process_stt_request(...) -> Result<String, CoreError> { ... } 

/// 处理音频输入并支持优雅关闭
#[instrument(skip(audio_rx, app_state, shutdown_rx))]
pub async fn audio_input_processor(
    mut audio_rx: mpsc::UnboundedReceiver<AudioMessage>,
    app_state: Arc<AppState>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    info!("Starting audio input processor task.");

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Shutdown signal received in audio processor. Exiting.");
                    break;
                }
            }

            maybe_message = audio_rx.recv() => {
                if let Some(message) = maybe_message {
                    // --- Check for active task BEFORE processing task-specific data ---
                    let active_task_info = app_state.get_active_task_info().await; // Use helper method

                    match message {
                        AudioMessage::Data(f32_data) => {
                            if let Some(task_info) = active_task_info {
                                // --- Task is active: Process audio data ---
                                trace!(task_id=%task_info.id, "Processing audio data chunk (size: {}) for active task.", f32_data.len());
                                // TODO: Implement VAD processing using f32_data
                                // TODO: If speech detected, save segment using task_info.task_dir
                                // TODO: If segment complete, potentially trigger STT (consider task_info.is_simulation?)
                                // Example placeholder:
                                // if !task_info.is_simulation {
                                //     // Perform actions only relevant for real tasks
                                // }
                            } else {
                                // --- No active task: Skip processing ---
                                trace!("No active task, skipping audio data processing (size: {}).", f32_data.len());
                            }
                        }
                        AudioMessage::Rms(rms_value) => {
                            // RMS is likely useful even without an active task (e.g., VU meter).
                            // Keep processing unconditional unless specified otherwise.
                            trace!("Received RMS value: {}", rms_value);
                            // TODO: Send RMS value somewhere (e.g., WebSocket broadcast) if needed.
                        }
                        AudioMessage::Error(error_msg) => {
                            // Log errors regardless of task status
                            error!("Received error from audio source: {}", error_msg);
                        }
                    }
                } else {
                    info!("Audio input channel closed. Exiting audio processor.");
                    break;
                }
            }
        }
    }
    debug!("Audio input processor task finished.");
}

// 或者，如果您已经有类似功能的函数，只需确保它是公开的并重命名为 audio_input_processor 