// Transmit Processing Logic (TTS, Queue Handling, PTT)

use super::error::CoreError; // Use the parent\'s error module
use super::state::AppState; // Use the parent\'s state module
use elfradio_types::{
    TxItem, PttSignal, AiConfig, AiProvider,
    LogEntry, LogDirection, LogContentType,
    TaskInfo, // 新增导入 TaskInfo
};
use elfradio_ai::TtsParams; // Removed AiError
use elfradio_hardware;

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::watch; // 新增导入
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;
use hound::{WavSpec, WavWriter, SampleFormat, Error as HoundError}; // 合并导入，移除 self 和重复项
use std::io::Cursor;
use tracing::{debug, error, info, warn, instrument};
use chrono::Utc;
use crate::logging;
use std::path::{Path, PathBuf};
use elfradio_db::insert_log_entry;

// Define a specific Result type alias for this module
type TxProcessingOutcome<T> = std::result::Result<T, CoreError>;

// ----------------------------------------------------------------------------
// Helper Functions (Should be defined before use)
// ----------------------------------------------------------------------------

/// Processes a single transmit item (e.g., TTS, play audio, PTT control).
/// This function is defined *before* tx_queue_processor.
async fn process_tx_item(
    item: TxItem,
    app_state: Arc<AppState>,
    task_info: &TaskInfo,
) -> TxProcessingOutcome<()> {
    let item_id = item.id();
    // Use task_info directly passed from the caller
    let task_dir = &task_info.task_dir;
    let task_id = task_info.id;
    let task_id_str = task_info.id.to_string();
    let is_simulation = task_info.is_simulation;

    info!(item_id = %item_id, task_id = %task_id_str, "Processing TX item for task.");
    
    // 获取数据库连接池
    let db_pool = &app_state.db_pool;

    // --- Get Hardware/Timing Config ---
    let serial_port = app_state.config.hardware.serial_port.as_deref();
    let ptt_signal_str = &app_state.config.hardware.ptt_signal;
    let ptt_pre_delay = app_state.config.timing.ptt_pre_delay_ms;
    let ptt_post_delay = app_state.config.timing.ptt_post_delay_ms;

    let ptt_signal: PttSignal = ptt_signal_str.parse().map_err(CoreError::PttSignalParseError)?;

    match item {
        TxItem::GeneratedVoice { id: _, audio_data, priority: _ } => {
            debug!(item_id = %item_id, task_id = %task_id_str, is_simulation, "Processing GeneratedVoice item");

            // --- Log TX Start to file (已有的代码) ---
            let start_entry = LogEntry {
                timestamp: Utc::now(),
                direction: LogDirection::Outgoing,
                content_type: LogContentType::Status,
                content: format!("Transmission started (Item ID: {})", item_id),
            };
            if let Err(e) = logging::write_log_entry(task_dir, &start_entry) {
                error!(task_id = %task_id_str, item_id = %item_id, "Failed to write TX Start log entry: {:?}", e);
            }
            
            // --- 添加: Log TX Start to database ---
            if let Err(e) = insert_log_entry(db_pool, task_id, &start_entry).await {
                error!(task_id = %task_id_str, item_id = %item_id, "Failed to insert TX Start log entry into database: {:?}", e);
            } else {
                debug!(task_id = %task_id_str, item_id = %item_id, "TX Start log entry inserted into database.");
            }

            // --- PTT and Audio Transmission (Conditional on Simulation) ---
            let mut final_send_result = Ok(());
            let mut final_ptt_off_result = Ok(());

            if !is_simulation {
                info!(item_id=%item_id, task_id=%task_id_str, "Performing real hardware transmission.");
                let port = serial_port.ok_or(CoreError::PttPortNotConfigured)?;

                // Activate PTT
                elfradio_hardware::set_ptt(port, ptt_signal, true, ptt_pre_delay, ptt_post_delay).await?;
                sleep(Duration::from_millis(ptt_pre_delay)).await;

                // Send audio data
                final_send_result = if let Some(sender) = app_state.audio_output_sender.lock().await.as_ref() {
                    let estimated_duration_secs = audio_data.len() as f32 / 16000.0; // Assuming 16kHz
                    let estimated_duration = Duration::from_secs_f32(estimated_duration_secs);
                    debug!(item_id = %item_id, task_id=%task_id_str, duration_ms = estimated_duration.as_millis(), "Sending audio data...");

                    sender.send(audio_data).map_err(|_| CoreError::AudioChannelClosed)?;
                    sleep(estimated_duration).await; // Sleep for estimated duration
                    Ok(())
                } else {
                    error!(item_id = %item_id, task_id=%task_id_str, "Audio output sender is not available.");
                    Err(CoreError::AudioChannelClosed)
                };

                // Deactivate PTT
                final_ptt_off_result = elfradio_hardware::set_ptt(
                    port, ptt_signal, false, ptt_pre_delay, ptt_post_delay
                )
                .await
                .map_err(CoreError::from);

                sleep(Duration::from_millis(ptt_post_delay)).await;
            } else {
                info!(item_id=%item_id, task_id=%task_id_str, "Simulation mode: Skipping hardware PTT and audio output.");
                // Simulate delay for timing consistency if needed
                let estimated_duration_secs = audio_data.len() as f32 / 16000.0;
                let simulated_total_delay = Duration::from_secs_f32(estimated_duration_secs)
                    + Duration::from_millis(ptt_pre_delay)
                    + Duration::from_millis(ptt_post_delay);
                sleep(simulated_total_delay).await;
            }

            // --- Log TX End to file (已有的代码) ---
            let end_entry = LogEntry {
                timestamp: Utc::now(),
                direction: LogDirection::Internal,
                content_type: LogContentType::Status,
                content: format!("Transmission finished (Item ID: {}){}", item_id, if is_simulation {" (Simulated)"} else {""}),
            };
            if let Err(e) = logging::write_log_entry(task_dir, &end_entry) {
                error!(task_id=%task_id_str, item_id = %item_id, "Failed to write TX End log entry: {:?}", e);
            }
            
            // --- 添加: Log TX End to database ---
            if let Err(e) = insert_log_entry(db_pool, task_id, &end_entry).await {
                error!(task_id = %task_id_str, item_id = %item_id, "Failed to insert TX End log entry into database: {:?}", e);
            } else {
                debug!(task_id = %task_id_str, item_id = %item_id, "TX End log entry inserted into database.");
            }

            // Check results after logging end
            final_send_result?;
            final_ptt_off_result?;

            debug!(item_id = %item_id, task_id=%task_id_str, "Finished processing GeneratedVoice item");
        }

        TxItem::ManualText { id, text, priority } | TxItem::AiReply { id, text, priority } => {
            // TTS should happen regardless of simulation mode, as the result is queued.
            // The is_simulation check happens when the GeneratedVoice item is processed.
            let text_to_speak = text;
            info!(item_id = %id, task_id=%task_id_str, "Received text item for TTS: '{}'", text_to_speak);

            let tts_params = construct_tts_params(&app_state.config.ai_settings);
            debug!(item_id = %id, task_id=%task_id_str, "Constructed TTS Params: {:?}", tts_params);

            // 获取 Option<Arc<dyn AiClient...>> 的读锁
            let ai_client_guard = app_state.ai_client.read().await;

            let audio_bytes = if let Some(client) = ai_client_guard.as_ref() {
                // 如果客户端存在，调用其方法
                client.text_to_speech(&text_to_speak, &tts_params).await.map_err(|e| {
                    error!(item_id = %id, task_id=%task_id_str, "TTS request failed: {:?}", e);
                    CoreError::AiRequestFailed(format!("TTS failed: {}", e)) // 将 AiError 映射到 CoreError
                })?
            } else {
                // 如果客户端为 None（未配置），返回新的特定错误
                warn!(item_id = %id, task_id=%task_id_str, "Attempted to call TTS in process_tx_item, but AI provider is not configured.");
                return Err(CoreError::AiNotConfigured); // 使用新的专用错误类型
            };
            
            let (audio_f32, _wav_spec): (Vec<f32>, WavSpec) = decode_wav_data(&audio_bytes)?;
            debug!("Decoded WAV data, samples count: {}", audio_f32.len());

            let generated_voice_item = TxItem::GeneratedVoice { id, audio_data: audio_f32, priority };
            info!(item_id = %id, task_id=%task_id_str, "Created GeneratedVoice item from TTS result.");

            if let Err(e) = app_state.tx_queue.send(generated_voice_item) {
                let failed_item_id = e.0.id();
                error!(item_id = %failed_item_id, task_id=%task_id_str, "Failed to re-queue GeneratedVoice item: {}", e);
                return Err(CoreError::TxQueueSendError(format!("Failed to send item {} to tx queue", failed_item_id)));
            } else {
                info!(item_id = %id, task_id=%task_id_str, "Successfully re-queued item as GeneratedVoice.");
            }
        }

        TxItem::ManualVoice { id, path, priority: _ } => {
             warn!(item_id = %id, task_id=%task_id_str, ?path, "Processing ManualVoice item - Not implemented yet.");
             // TODO: Implement logic (consider simulation flag here too if needed)
        }
    }

    Ok(())
}

/// Decodes WAV audio data (bytes) into a vector of f32 samples.
pub fn decode_wav_data(wav_data: &[u8]) -> TxProcessingOutcome<(Vec<f32>, WavSpec)> {
    let cursor = Cursor::new(wav_data);
    let mut reader = hound::WavReader::new(cursor).map_err(CoreError::AudioDecodeError)?;
    let spec = reader.spec();

    // Validate Spec if necessary (e.g., check sample rate, channels)
    if spec.channels != 1 {
        warn!("Expected mono audio for decoding, got {} channels. Will attempt to process first channel.", spec.channels);
        // Or return Err(CoreError::AudioDecodeError(HoundError::Unsupported)) if strict mono is required
    }

    let samples_f32: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            // Read samples based on bits_per_sample
            match spec.bits_per_sample {
                16 => reader
                    .samples::<i16>()
                    .map(|s| s.map(|sample| sample as f32 / i16::MAX as f32))
                    .collect::<Result<Vec<_>, _>>()?,
                8 => reader // Hound reads 8-bit PCM as u8
                    .samples::<i8>() // Read as i8 assuming hound handles the unsigned->signed mapping
                    .map(|s| s.map(|sample| sample as f32 / i8::MAX as f32)) // Normalize i8
                    .collect::<Result<Vec<_>, _>>()?,
                24 => {
                     error!("24-bit WAV decoding not implemented yet in decode_wav_data.");
                     return Err(CoreError::AudioDecodeError(HoundError::Unsupported));
                }
                 32 => reader
                    .samples::<i32>()
                    .map(|s| s.map(|sample| sample as f32 / i32::MAX as f32))
                    .collect::<Result<Vec<_>, _>>()?,
                _ => {
                    return Err(CoreError::AudioDecodeError(HoundError::Unsupported));
                }
            }
        }
        hound::SampleFormat::Float => {
             // Hound reads 32-bit float samples directly as f32
            if spec.bits_per_sample == 32 {
                reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?
            } else {
                 return Err(CoreError::AudioDecodeError(HoundError::Unsupported));
            }
        }
    };

    Ok((samples_f32, spec))
}

// 帮助函数：构造 TTS 参数 (Moved from processing.rs)
fn construct_tts_params(ai_config: &AiConfig) -> TtsParams {
    let active_provider = ai_config.provider.clone();

    // Determine voice ID based on provider
    let voice_id = match active_provider {
        Some(AiProvider::GoogleGemini) => ai_config.google.as_ref()
            .and_then(|g| g.tts_voice.clone())
            .unwrap_or_else(|| {
                warn!("Google TTS voice not specified, using default en-US-Standard-A ");
                "en-US-Standard-A ".to_string()
            }),
        Some(AiProvider::StepFunTTS) => ai_config.stepfun_tts.as_ref()
            .and_then(|_| None)
            .unwrap_or_else(|| {
                warn!("StepFun TTS voice not specified, using default wenrounvsheng ");
                "wenrounvsheng ".to_string()
            }),
        Some(AiProvider::OpenAICompatible) => ai_config.openai_compatible.as_ref()
             .and_then(|o| o.preferred_model.clone()) // Or maybe a specific tts_model field?
             .unwrap_or_else(|| {
                 warn!("OpenAI-compatible TTS model/voice not specified, using default tts-1");
                 "tts-1".to_string() // Assuming model acts as voice identifier here
             }),
        None => {
             warn!("No AI provider specified in config, using placeholder DefaultVoice for TTS params.");
            "DefaultVoice".to_string()
        },
    };

    // Determine language code (primarily for Google)
    let language_code = match active_provider {
         Some(AiProvider::GoogleGemini) => ai_config.google.as_ref()
             .and_then(|g| g.stt_language.clone()), // Reuse STT language for TTS if appropriate
         _ => None, // Other providers might infer from voice or not need it
    };

    // Determine speed (using temperature as a proxy for now, might need dedicated config)
    let speed: Option<f32> = ai_config.temperature; // Consider renaming config field if it's for TTS speed

    // Determine volume (no direct mapping in current config)
    let volume: Option<f32> = None; // Add dedicated config field if needed

    TtsParams {
        voice_id,
        language_code,
        speed,
        volume,
        output_format: "wav".to_string() // Request WAV from TTS service
    }
}

// ----------------------------------------------------------------------------
// Transmit Queue Processor
// ----------------------------------------------------------------------------

/// Processes items from the transmit queue, handling TTS and PTT control,
/// and supports graceful shutdown. Checks for active task before processing.
pub async fn tx_queue_processor(
    mut tx_rx: mpsc::UnboundedReceiver<TxItem>,
    app_state: Arc<AppState>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    info!("Starting transmit queue processor task.");

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Shutdown signal received in TX processor. Exiting.");
                    break;
                }
            }

            maybe_item = tx_rx.recv() => {
                if let Some(item) = maybe_item {
                    // --- Check for active task BEFORE processing ---
                    let active_task_info = app_state.get_active_task_info().await;

                    if let Some(task_info) = active_task_info {
                        // --- Task is active: Proceed with processing ---
                        debug!(item_id = %item.id(), task_id=%task_info.id, "Processing TX item for active task.");
                        let mut is_transmitting_guard = app_state.is_transmitting.lock().await;
                        if *is_transmitting_guard {
                            warn!(item_id = %item.id(), task_id=%task_info.id, "Already transmitting, skipping TX item.");
                            continue;
                        }
                        *is_transmitting_guard = true;
                        drop(is_transmitting_guard);

                        let item_id = item.id();
                        // Pass the retrieved task_info to process_tx_item
                        let result = process_tx_item(item, app_state.clone(), &task_info).await;

                        *app_state.is_transmitting.lock().await = false;

                        if let Err(e) = result {
                            error!(item_id = %item_id, task_id=%task_info.id, "Error processing TX item: {:?}", e);
                        } else {
                            info!(item_id = %item_id, task_id=%task_info.id, "Finished processing TX item.");
                        }
                        // sleep(Duration::from_secs(1)).await; // Optional delay
                    } else {
                        // --- No active task: Drop the item ---
                        warn!(item_id = %item.id(), "No active task, dropping TX item.");
                        // Optionally, could re-queue with low priority or store somewhere?
                    }
                } else {
                    info!("TX queue channel closed. Exiting TX processor.");
                    break;
                }
            }
        }
    }
    info!("TX Queue Processor task finished.");
}

// ----------------------------------------------------------------------------
// Public API for Queueing Text (Moved from lib.rs)
// ----------------------------------------------------------------------------

/// Generates speech audio from text using TTS and queues it for transmission.
/// This is intended for use cases where TTS is triggered directly, bypassing
/// the standard AI reply flow (e.g., manual text input).
#[instrument(skip(app_state, text_to_speak), fields(text_len = text_to_speak.len()))]
pub async fn queue_text_for_transmission(
    app_state: Arc<AppState>,
    text_to_speak: String,
) -> Result<(), CoreError> {
    info!("Queuing text for transmission: {}", text_to_speak);
    
    let task_id_opt: Option<Uuid> = {
        let task_guard = app_state.active_task.lock().await;
        task_guard.as_ref().map(|info| info.id)
    };
    let db_pool = &app_state.db_pool;
    let task_dir_opt: Option<PathBuf> = {
         let task_guard = app_state.active_task.lock().await;
         task_guard.as_ref().map(|info| info.task_dir.clone())
    };

    if let (Some(task_id), Some(task_dir)) = (task_id_opt, task_dir_opt.as_ref()) {
        let entry = LogEntry {
            timestamp: Utc::now(),
            direction: LogDirection::Outgoing,
            content_type: LogContentType::Text,
            content: text_to_speak.clone(),
        };
        if let Err(e) = insert_log_entry(db_pool, task_id, &entry).await {
            error!(task_id = %task_id, "Failed to insert SendText log entry into database: {:?}", e);
        } else {
            debug!(task_id = %task_id, "SendText log entry inserted into database.");
        }

        let tts_params = construct_tts_params(&app_state.config.ai_settings);
        debug!("Constructed TTS Params: {:?}", tts_params);

        // 获取 Option<Arc<dyn AiClient...>> 的读锁
        let ai_client_guard = app_state.ai_client.read().await;

        let audio_bytes = if let Some(client) = ai_client_guard.as_ref() {
            // 如果客户端存在，调用其方法
            client.text_to_speech(&text_to_speak, &tts_params).await.map_err(|e| {
                error!(item_id = %task_id, task_id=%task_id, "TTS request failed: {:?}", e);
                CoreError::AiRequestFailed(format!("TTS failed: {}", e)) // 将 AiError 映射到 CoreError
            })?
        } else {
            // 如果客户端为 None（未配置），返回新的特定错误
            warn!(item_id = %task_id, task_id=%task_id, "Attempted to call TTS in process_tx_item, but AI provider is not configured.");
            return Err(CoreError::AiNotConfigured); // 使用新的专用错误类型
        };
        
        let (audio_f32, _wav_spec): (Vec<f32>, WavSpec) = decode_wav_data(&audio_bytes)?;
        debug!("Decoded WAV data, samples count: {}", audio_f32.len());

        let sample_rate = app_state.config.hardware.input_sample_rate;
        debug!("Using sample rate: {}", sample_rate);

        let filename = format!("{}.wav", Uuid::new_v4());
        let audio_file_path = task_dir.join(&filename);
        
        match save_wav_file(&audio_file_path, &audio_f32, sample_rate).await {
             Ok(_) => {
                 info!(task_id = %task_id, "Saved TTS audio to {:?}", audio_file_path);
                 let audio_entry = LogEntry {
                     timestamp: Utc::now(),
                     direction: LogDirection::Outgoing,
                     content_type: LogContentType::Audio,
                     content: filename,
                 };
                 if let Err(e) = insert_log_entry(db_pool, task_id, &audio_entry).await {
                     error!(task_id = %task_id, "Failed to insert TTS Audio log entry: {:?}", e);
                 } else {
                     debug!(task_id = %task_id, "TTS Audio log entry inserted.");
                 }
             }
             Err(e) => {
                  error!(task_id = %task_id, "Failed to save TTS audio file: {:?}", e);
                  return Err(e);
             }
        }

        let tx_item = TxItem::GeneratedVoice {
             id: Uuid::new_v4(),
             audio_data: audio_f32,
             priority: 5,
        };
        debug!("Created TxItem: {:?}", tx_item);

        app_state.tx_queue.send(tx_item)
            .map_err(|e| CoreError::TxQueueSendError(format!("Failed to send to TX queue: {}", e)))?;
        debug!("Successfully queued TxItem for transmission.");

        Ok(())
    } else {
        warn!("No active task found when trying to queue text for transmission.");
        Err(CoreError::NoTaskRunning)
    }
}

// ----------------------------------------------------------------------------
// Utility Functions
// ----------------------------------------------------------------------------

/// Helper function to get the currently configured active TTS voice ID.
pub fn get_active_tts_voice(ai_config: &AiConfig) -> String {
    // Reuses the logic from construct_tts_params to find the voice ID
    let params = construct_tts_params(ai_config);
    params.voice_id
}

// 添加辅助函数用于保存 WAV 文件
#[instrument(skip(path, samples))]
async fn save_wav_file(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), CoreError> {
    info!("Saving WAV file to: {:?}", path);
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let file = std::fs::File::create(path).map_err(CoreError::IoError)?;
    let mut writer = WavWriter::new(file, spec).map_err(|e| CoreError::AudioError(e.to_string()))?;

    for &sample in samples {
        let sample_i16 = (sample * i16::MAX as f32) as i16;
        writer.write_sample(sample_i16).map_err(|e| CoreError::AudioError(e.to_string()))?;
    }

    writer.finalize().map_err(|e| CoreError::AudioError(e.to_string()))?;
    debug!("Successfully saved WAV file: {:?}", path);
    Ok(())
} 