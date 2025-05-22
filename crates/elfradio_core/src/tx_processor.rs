// Transmit Processing Logic (TTS, Queue Handling, PTT)

use super::error::CoreError; // Use the parent\'s error module
use super::state::AppState; // Use the parent\'s state module
use elfradio_types::{
    TxItem, PttSignal, AiConfig, AiProvider,
    LogEntry, LogDirection, LogContentType,
    TaskInfo, // ADDED AiError for mapping
    WebSocketMessage, SystemServiceStatus, AiError, // Added for 5.7.6.1
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
use rubato::Resampler;
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
    // 首先记录尝试解码的音频数据长度
    debug!("Attempting to decode WAV data of length: {} bytes", wav_data.len());

    // 修改 WavReader 创建的错误处理，使用 {:?} 打印错误的 Debug 表示
    let mut reader = match hound::WavReader::new(Cursor::new(wav_data)) {
        Ok(r) => r,
        Err(e) => {
            let snippet_len = std::cmp::min(wav_data.len(), 64); // 记录前64个字节
            let snippet = &wav_data[..snippet_len];
            error!(
                "Hound: 无法读取 WAV 头部。错误详情: {:?}. 原始音频字节片段 (前 {} 字节): {:02X?}",
                e, snippet_len, snippet
            );
            return Err(CoreError::AudioDecodeError(e)); // 直接传递 HoundError
        }
    };

    let spec = reader.spec();

    // 验证音频规格
    if spec.channels != 1 {
        warn!("期望单声道音频，但收到 {} 个声道。将尝试处理第一个声道。", spec.channels);
    }

    let samples_f32: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            match spec.bits_per_sample {
                16 => {
                    match reader
                    .samples::<i16>()
                    .map(|s| s.map(|sample| sample as f32 / i16::MAX as f32))
                        .collect::<Result<Vec<_>, _>>()
                    {
                        Ok(samples) => samples,
                        Err(e) => {
                            let snippet_len = std::cmp::min(wav_data.len(), 64);
                            let snippet = &wav_data[..snippet_len];
                            error!(
                                "Hound: 无法读取 i16 WAV 采样。错误详情: {:?}. 原始音频字节片段 (前 {} 字节): {:02X?}",
                                e, snippet_len, snippet
                            );
                            return Err(CoreError::AudioDecodeError(e)); // 直接传递 HoundError
                        }
                    }
                },
                8 => {
                    match reader
                        .samples::<i8>()
                        .map(|s| s.map(|sample| sample as f32 / i8::MAX as f32))
                        .collect::<Result<Vec<_>, _>>()
                    {
                        Ok(samples) => samples,
                        Err(e) => {
                            let snippet_len = std::cmp::min(wav_data.len(), 64);
                            let snippet = &wav_data[..snippet_len];
                            error!(
                                "Hound: 无法读取 i8 WAV 采样。错误详情: {:?}. 原始音频字节片段 (前 {} 字节): {:02X?}",
                                e, snippet_len, snippet
                            );
                            return Err(CoreError::AudioDecodeError(e)); // 直接传递 HoundError
                        }
                    }
                },
                24 => {
                    error!("24位 WAV 解码尚未在 decode_wav_data 中实现。");
                     return Err(CoreError::AudioDecodeError(HoundError::Unsupported));
                },
                32 => {
                    match reader
                    .samples::<i32>()
                    .map(|s| s.map(|sample| sample as f32 / i32::MAX as f32))
                        .collect::<Result<Vec<_>, _>>()
                    {
                        Ok(samples) => samples,
                        Err(e) => {
                            let snippet_len = std::cmp::min(wav_data.len(), 64);
                            let snippet = &wav_data[..snippet_len];
                            error!(
                                "Hound: 无法读取 i32 WAV 采样。错误详情: {:?}. 原始音频字节片段 (前 {} 字节): {:02X?}",
                                e, snippet_len, snippet
                            );
                            return Err(CoreError::AudioDecodeError(e)); // 直接传递 HoundError
                        }
                    }
                },
                _ => {
                    error!("不支持的位深度: {} 位", spec.bits_per_sample);
                    return Err(CoreError::AudioDecodeError(HoundError::Unsupported));
                }
            }
        },
        hound::SampleFormat::Float => {
            if spec.bits_per_sample == 32 {
                match reader.samples::<f32>().collect::<Result<Vec<_>, _>>() {
                    Ok(samples) => samples,
                    Err(e) => {
                        let snippet_len = std::cmp::min(wav_data.len(), 64);
                        let snippet = &wav_data[..snippet_len];
                        error!(
                            "Hound: 无法读取 f32 WAV 采样。错误详情: {:?}. 原始音频字节片段 (前 {} 字节): {:02X?}",
                            e, snippet_len, snippet
                        );
                        return Err(CoreError::AudioDecodeError(e)); // 直接传递 HoundError
                    }
                }
            } else {
                error!("不支持的浮点位深度: {} 位", spec.bits_per_sample);
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
#[instrument(skip(app_state, text_to_speak, log_entry_tx, status_update_tx), fields(text_len = text_to_speak.len()))]
pub async fn queue_text_for_transmission(
    app_state: Arc<AppState>,
    text_to_speak: String,
    log_entry_tx: &tokio::sync::mpsc::UnboundedSender<LogEntry>,      // New parameter
    status_update_tx: &tokio::sync::mpsc::UnboundedSender<WebSocketMessage> // New parameter
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

        // --- Determine TTS parameters (language_code, voice_name) for AuxServiceClient ---
        let lang_code_str: String;
        let voice_name_opt: Option<String>;

        // Borrow aux_service_settings from the app_state config
        let aux_provider_cfg = &app_state.config.aux_service_settings; // This is &AuxServiceConfig

        match aux_provider_cfg.provider.as_ref() {
            Some(elfradio_types::AuxServiceProvider::Google) => {
                let google_aux_params_cfg = &aux_provider_cfg.google; // This is &GoogleAuxConfig, which is part of AuxServiceConfig
                voice_name_opt = google_aux_params_cfg.tts_voice.clone();
                // Infer language from tts_voice (e.g., "en-US-Wavenet-D" -> "en-US")
                // Fallback to stt_language from the same GoogleAuxConfig, then a hardcoded default.
                lang_code_str = voice_name_opt.as_ref()
                    .and_then(|v_name| {
                        // Attempt to extract language part like "en-US" from "en-US-Wavenet-D"
                        let parts: Vec<&str> = v_name.splitn(3, '-').take(2).collect();
                        if parts.len() == 2 {
                            Some(parts.join("-"))
                        } else {
                            None // Could not reliably extract language from voice name
                        }
                    })
                    .filter(|s: &String| !s.is_empty())
                    .or_else(|| google_aux_params_cfg.stt_language.clone()) // stt_language is also Option<String>
                    .unwrap_or_else(|| {
                        warn!(task_id = %task_id, "Google Aux TTS: language_code could not be determined from tts_voice ('{:?}') or stt_language ('{:?}') in aux_service_settings.google. Defaulting to 'en-US'.", voice_name_opt, google_aux_params_cfg.stt_language);
                        "en-US".to_string()
                    });
                if voice_name_opt.is_none() {
                    warn!(task_id = %task_id, "Google Aux TTS: voice_name not configured in aux_service_settings.google.tts_voice. Aux client will use its default for lang '{}'.", lang_code_str);
                }
                info!(task_id = %task_id, "Using Google Aux TTS: lang_code='{}', voice_name='{:?}'", lang_code_str, voice_name_opt);
            }
            Some(elfradio_types::AuxServiceProvider::Aliyun) => {
                voice_name_opt = Some("Aiyue".to_string()); 
                lang_code_str = "zh-CN".to_string(); 
                info!(task_id = %task_id, "Using Aliyun Aux TTS: lang_code='{}', voice_name='{:?}'", lang_code_str, voice_name_opt);
            }
            None | Some(_) => { 
                let provider_display_name = match aux_provider_cfg.provider.as_ref() {
                    Some(p_val) => format!("{:?}", p_val), 
                    None => "None".to_string(),
                };
                warn!(task_id = %task_id, "Auxiliary TTS provider is {} or not fully supported for parameter extraction. Defaulting TTS params to en-US, no specific voice.", provider_display_name);
                lang_code_str = "en-US".to_string();
                voice_name_opt = None;
            }
        }
        debug!(task_id = %task_id, "Determined TTS params: lang_code_str='{}', voice_name_opt='{:?}'", lang_code_str, voice_name_opt);
        
        let lang_code_ref: &str = &lang_code_str;
        let voice_name_ref: Option<&str> = voice_name_opt.as_deref();

        debug!(task_id = %task_id, "Attempting TTS via aux_client with lang_code: '{}', voice_name: {:?}", lang_code_ref, voice_name_ref);

        let audio_bytes: Vec<u8>; 

        let aux_client_guard = app_state.aux_client.read().await;

        if let Some(client) = aux_client_guard.as_ref() {
            match client.text_to_speech(&text_to_speak, lang_code_ref, voice_name_ref).await {
                Ok(bytes) => {
                    audio_bytes = bytes;
                    info!(task_id = %task_id, "TTS call successful via aux_client, received {} audio bytes.", audio_bytes.len());
                }
                Err(ai_error) => {
                    // --- Enhanced Error Handling for TTS Failure (Step 5.7.6.1) ---
                    let determined_tts_status = match &ai_error {
                        AiError::AuthenticationError(_) | AiError::ApiError { status: 401, .. } | AiError::ApiError { status: 403, .. } => {
                            SystemServiceStatus::Warning
                        }
                        AiError::ApiError { status: 429, .. } => {
                            SystemServiceStatus::Warning
                        }
                        AiError::RequestError(_) | AiError::ApiError { status: 500..=599, .. } | AiError::ClientError(_) => {
                            SystemServiceStatus::Error
                        }
                        _ => SystemServiceStatus::Error,
                    };

                    let log_message = format!(
                        "TTS Service Runtime Error (Provider: {:?}): Failed to convert text to speech. Status determined: {:?}. Details: {:?}",
                        app_state.config.aux_service_settings.provider,
                        determined_tts_status,
                        ai_error
                    );
                    error!(task_id = %task_id, "{}", log_message); 

                    let tts_error_log_entry = LogEntry {
                        timestamp: Utc::now(),
                        direction: LogDirection::Internal,
                        content_type: LogContentType::Status,
                        content: log_message,
                    };
                    if log_entry_tx.send(tts_error_log_entry).is_err() {
                        error!(task_id = %task_id, "Failed to send TTS runtime error log entry via MPSC channel.");
                    }

                    let tts_status_update_msg = WebSocketMessage::TtsStatusUpdate(determined_tts_status);
                    if status_update_tx.send(tts_status_update_msg).is_err() {
                        error!(task_id = %task_id, "Failed to send TtsStatusUpdate via MPSC channel for runtime error.");
                    }
                    // --- End Enhanced Error Handling ---
                    return Err(CoreError::from(ai_error)); 
                }
            }
        } else {
            error!(task_id = %task_id, "TTS failed: Auxiliary service (aux_client) is not configured.");
            return Err(CoreError::AuxServiceNotConfigured(
                "TTS service is not available because no auxiliary service provider is configured in aux_service_settings.".to_string()
            ));
        }

        // 声明处理后的音频数据和WAV规格
        let audio_f32: Vec<f32>;
        let mut wav_spec_for_saving: WavSpec;

        // --- 根据不同的辅助服务提供商处理音频数据 ---
        if app_state.config.aux_service_settings.provider == Some(elfradio_types::AuxServiceProvider::Aliyun) {
            info!(
                task_id = %task_id,
                "Processing raw PCM audio data ({} bytes) from Aliyun TTS...",
                audio_bytes.len()
            );

            // --- 保存原始 Aliyun PCM 字节数据 (扩展名 .pcm) ---
            let raw_filename = format!("raw_aliyun_tts_{}.pcm", Uuid::new_v4());
            let raw_audio_file_path = task_dir.join(&raw_filename);
            
            match tokio::fs::write(&raw_audio_file_path, &audio_bytes).await {
                Ok(_) => {
                    info!(
                        task_id = %task_id,
                        "Successfully saved raw Aliyun PCM bytes ({} bytes) to {:?}",
                        audio_bytes.len(),
                        raw_audio_file_path
                    );
                    
                    // 记录原始PCM文件到数据库
                    let raw_audio_entry = LogEntry {
                        timestamp: Utc::now(),
                        direction: LogDirection::Outgoing,
                        content_type: LogContentType::Audio,
                        content: format!("Raw PCM audio: {}", raw_filename),
                    };
                    if let Err(e) = insert_log_entry(db_pool, task_id, &raw_audio_entry).await {
                        error!(
                            task_id = %task_id,
                            "Failed to insert raw PCM audio log entry: {:?}",
                            e
                        );
                    }
                }
                Err(e) => {
                    error!(
                        task_id = %task_id,
                        "Failed to save raw Aliyun PCM bytes to {:?}: {}",
                        raw_audio_file_path,
                        e
                    );
                    // 不返回错误，继续处理
                }
            }

            // --- 将原始 PCM (Vec<u8>) 转换为 Vec<f32> ---
            // 确保字节长度是偶数（处理16位样本）
            if audio_bytes.len() % 2 != 0 {
                let error_message = format!(
                    "Invalid PCM data length from Aliyun for i16 conversion: {} bytes (expected even number)", 
                    audio_bytes.len()
                );
                error!(
                    task_id = %task_id,
                    "Aliyun PCM data has odd length ({}), cannot convert to i16 samples.",
                    audio_bytes.len()
                );
                return Err(CoreError::AudioError(error_message));
            }

            // 将字节按照16位有符号整数（小端序）处理并归一化为f32
            audio_f32 = audio_bytes
                .chunks_exact(2)
                .map(|chunk| {
                    let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                    sample as f32 / i16::MAX as f32
                })
                .collect();

            // 创建用于保存的 WavSpec
            wav_spec_for_saving = WavSpec {
                channels: 1,
                sample_rate: 16000, // 假定Aliyun PCM采样率为16kHz
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            };

            info!(
                task_id = %task_id,
                "Converted Aliyun PCM ({} bytes) to {} f32 samples. Assumed spec for saving: {:?}",
                audio_bytes.len(),
                audio_f32.len(),
                wav_spec_for_saving
            );
        } else {
            // 对于Google或其他返回完整WAV的提供商
            info!(
                task_id = %task_id,
                "Attempting to decode WAV data ({} bytes) obtained from TTS (e.g., Google)...",
                audio_bytes.len()
            );

            // 记录音频字节的前几个字节用于调试
            let snippet_len = std::cmp::min(audio_bytes.len(), 32);
            debug!(
                task_id = %task_id,
                "Audio bytes snippet (first {} bytes): {:?}",
                snippet_len,
                &audio_bytes[..snippet_len]
            );
            
            // 使用现有的 decode_wav_data 函数来处理WAV格式
            let (decoded_f32_samples, original_wav_spec) = match decode_wav_data(&audio_bytes) {
                Ok(result) => result,
                Err(e) => {
                    error!(
                        task_id = %task_id,
                        "Audio decoding error after TTS: {:?}. Full audio_bytes length: {}",
                        e,
                        audio_bytes.len()
                    );
                    return Err(e);
                }
            };

            debug!(
                task_id = %task_id,
                "Decoded WAV data, f32 samples count: {}, original spec: {:?}",
                decoded_f32_samples.len(),
                original_wav_spec
            );

            // --- 开始：重采样逻辑 ---
            let target_sample_rate = 16000u32;
            if original_wav_spec.sample_rate != target_sample_rate {
                info!(
                    task_id = %task_id,
                    "Resampling audio from {} Hz to {} Hz...",
                    original_wav_spec.sample_rate,
                    target_sample_rate
                );

                // 检查通道数（预期为单声道）
                if original_wav_spec.channels != 1 {
                    warn!(
                        task_id = %task_id,
                        "Attempting to resample audio with {} channels. Rubato SincFixedIn expects input as Vec<Vec<f32>> where outer Vec is for channels. Assuming first channel if stereo, or proceed if mono.",
                        original_wav_spec.channels
                    );
                    // 目前，我们假设 audio_f32 已经是单声道，
                    // 或者如果是立体声，我们在此之前只会使用第一个通道。
                    // 如果 original_wav_spec.channels > 1，且 audio_f32 包含交错数据，
                    // 则需要先将其解交错为 Vec<Vec<f32>>。
                    // 让我们假设 audio_f32 已经是表示单个通道的 Vec<f32>。
                }
                let num_channels = 1usize; // 我们处理的是单声道音频
                
                // 计算重采样比率
                let resample_ratio = target_sample_rate as f64 / original_wav_spec.sample_rate as f64;
                
                // 定义重采样器的块大小
                let resampler_chunk_size = 1024; // 重采样器的常见块大小
                
                // 创建 SincInterpolationParameters
                let sinc_params = rubato::SincInterpolationParameters {
                    sinc_len: 128,
                    f_cutoff: 0.95,
                    interpolation: rubato::SincInterpolationType::Linear,
                    oversampling_factor: 128,
                    window: rubato::WindowFunction::BlackmanHarris2,
                };
                
                // 创建 SincFixedIn 重采样器实例
                let mut resampler = match rubato::SincFixedIn::<f32>::new(
                    resample_ratio,
                    2.0, // max_resample_ratio_relative（对于降采样，2.0 是安全的）
                    sinc_params,
                    resampler_chunk_size, // 这是重采样器期望的输入块大小（以帧为单位）
                    num_channels,        // 音频通道数
                ) {
                    Ok(r) => r,
                    Err(e) => {
                        error!(task_id = %task_id, "Failed to create rubato resampler: {:?}", e);
                        return Err(CoreError::AudioError(format!("Failed to create resampler: {}", e)));
                    }
                };
                
                // 为重采样后的所有帧准备输出向量
                let mut all_resampled_frames: Vec<f32> = Vec::with_capacity(
                    (decoded_f32_samples.len() as f64 * resample_ratio).ceil() as usize
                );
                
                debug!(
                    task_id = %task_id,
                    "Resampler initialized. Original frames: {}, Target ratio: {}. Starting corrected chunked processing.",
                    decoded_f32_samples.len(),
                    resample_ratio
                );
                
                // --- 开始：修正后的完整片段重采样逻辑 ---
                let mut input_cursor = 0;
                // temp_out_buffer 用于单次调用 process_into_buffer
                let mut temp_out_buffer: Vec<Vec<f32>> = vec![vec![0.0f32; resampler.output_frames_max()]; num_channels];

                // 处理所有完整的块
                while input_cursor + resampler_chunk_size <= decoded_f32_samples.len() {
                    let input_chunk_data: Vec<f32> = decoded_f32_samples[input_cursor..input_cursor + resampler_chunk_size].to_vec();
                    // 为单声道创建包含一个 Vec 的数组
                    let waves_in_chunk: [Vec<f32>; 1] = [input_chunk_data];

                    match resampler.process_into_buffer(&waves_in_chunk, &mut temp_out_buffer, None) {
                        Ok((_consumed, produced)) => {
                            if produced > 0 {
                                // 从 temp_out_buffer 的第一个通道收集结果
                                all_resampled_frames.extend_from_slice(&temp_out_buffer[0][..produced]);
                            }
                        }
                        Err(e) => {
                            error!(task_id=%task_id, "Error during full chunk resampling: {:?}", e);
                            return Err(CoreError::AudioError(format!("Resampling error: {}", e)));
                        }
                    }
                    input_cursor += resampler_chunk_size;
                }

                // 处理最后一个（可能不完整的）块，如果还有剩余的实际样本
                if input_cursor < decoded_f32_samples.len() {
                    let remaining_input_frames = decoded_f32_samples.len() - input_cursor;
                    debug!(task_id=%task_id, "Processing last partial chunk of {} frames.", remaining_input_frames);
                    let mut last_input_chunk_data: Vec<f32> = decoded_f32_samples[input_cursor..].to_vec();
                    // 用零填充以构成一个完整的块，供 SincFixedIn 处理
                    last_input_chunk_data.resize(resampler_chunk_size, 0.0f32);
                    let waves_in_last_chunk: [Vec<f32>; 1] = [last_input_chunk_data];

                    match resampler.process_into_buffer(&waves_in_last_chunk, &mut temp_out_buffer, None) {
                        Ok((_consumed, produced)) => {
                            debug!(task_id=%task_id, "Last chunk processed, produced {} frames.", produced);
                            // 我们只关心由 *实际* 剩余输入产生的帧，而不是由填充产生的。
                            // `produced` 应该反映了这一点。
                            if produced > 0 {
                                all_resampled_frames.extend_from_slice(&temp_out_buffer[0][..produced]);
                            }
                        }
                        Err(e) => {
                            error!(task_id=%task_id, "Error processing last chunk: {:?}", e);
                            return Err(CoreError::AudioError(format!("Resampling last chunk error: {}", e)));
                        }
                    }
                }

                // --- 开始：修正后的冲洗重采样器逻辑 ---
                debug!(task_id = %task_id, "Flushing resampler (refined)...");
                // 用于冲洗的输入：一个大小符合重采样器预期的填零块。
                // 注意：SincFixedIn 期望冲洗块也具有与处理时相同的块大小
                let flush_input_chunk_data: Vec<f32> = vec![0.0f32; resampler_chunk_size];
                let waves_in_flush_chunk: [Vec<f32>; 1] = [flush_input_chunk_data]; // 单声道

                const MAX_FLUSH_ITERATIONS: usize = 10; // 安全退出：最大冲洗迭代次数
                let mut flush_iterations = 0;
                let mut total_flushed_frames_in_this_phase = 0;

                loop {
                    if flush_iterations >= MAX_FLUSH_ITERATIONS {
                        warn!(
                            task_id = %task_id,
                            "Resampler flushing reached max iterations ({}), breaking loop. Total flushed in this phase: {}",
                            MAX_FLUSH_ITERATIONS,
                            total_flushed_frames_in_this_phase
                        );
                        break;
                    }

                    match resampler.process_into_buffer(&waves_in_flush_chunk, &mut temp_out_buffer, None) {
                        Ok((_input_frames_consumed, output_frames_produced)) => {
                            if output_frames_produced > 0 {
                                // 从 temp_out_buffer 的第一个通道收集结果
                                all_resampled_frames.extend_from_slice(&temp_out_buffer[0][..output_frames_produced]);
                                total_flushed_frames_in_this_phase += output_frames_produced;
                                debug!(
                                    task_id = %task_id,
                                    "Flushed {} frames from resampler (iteration {}).",
                                    output_frames_produced,
                                    flush_iterations + 1
                                );
                            } else {
                                // 没有更多帧被产生，冲洗被认为完成。
                                debug!(
                                    task_id = %task_id,
                                    "Resampler flushing complete (0 frames produced on iteration {}).",
                                    flush_iterations + 1
                                );
                                break;
                            }
                        }
                        Err(e) => {
                            error!(task_id = %task_id, "Error during resampler flush: {:?}", e);
                            return Err(CoreError::AudioError(format!("Error during resampler flush: {}", e)));
                        }
                    }
                    flush_iterations += 1;
                }
                // --- 结束：修正后的冲洗重采样器逻辑 ---

                // --- 使用重采样后的数据更新主要音频变量 ---
                // 将最终的重采样结果赋值给外部作用域的变量
                audio_f32 = all_resampled_frames;
                // 更新 WAV 规格以反映新的采样率
                // 注意：`wav_spec_for_saving` 在外部声明为 `mut`
                wav_spec_for_saving = original_wav_spec.clone(); // 克隆原始规格以保留其他信息
                wav_spec_for_saving.sample_rate = target_sample_rate; // 设置新的采样率

                info!(
                    task_id = %task_id,
                    "Resampling and flushing complete. Final audio data for saving/queuing: {} samples at {} Hz",
                    audio_f32.len(),
                    wav_spec_for_saving.sample_rate
                );
                // --- 结束：更新主要音频变量 ---

            } else { // 如果采样率已经是目标值
                info!(
                    task_id = %task_id,
                    "Audio already at target sample rate of {} Hz. No resampling needed.",
                    target_sample_rate
                );
                // 直接使用原始解码数据和规格
                audio_f32 = decoded_f32_samples;
                wav_spec_for_saving = original_wav_spec;
            }
            // --- 结束：重采样逻辑（整个 if/else 块） ---

            debug!(
                task_id = %task_id,
                "Final audio data ready for saving: {} samples, spec: {:?}",
                audio_f32.len(),
                wav_spec_for_saving
            );
        }

        // --- 保存处理后的音频为标准WAV文件 ---
        let filename = format!("processed_tts_{}.wav", Uuid::new_v4());
        let audio_file_path = task_dir.join(&filename);
        
        match save_wav_file(&audio_file_path, &audio_f32, wav_spec_for_saving.sample_rate).await {
             Ok(_) => {
                info!(task_id = %task_id, "Successfully saved processed TTS audio as WAV to {:?}", audio_file_path);
                 let audio_entry = LogEntry {
                     timestamp: Utc::now(),
                     direction: LogDirection::Outgoing,
                     content_type: LogContentType::Audio,
                     content: filename,
                 };
                 if let Err(e) = insert_log_entry(db_pool, task_id, &audio_entry).await {
                     error!(task_id = %task_id, "Failed to insert TTS Audio log entry: {:?}", e);
                 } else {
                    debug!(task_id = %task_id, "TTS Audio log entry inserted into database.");
                 }
             }
             Err(e) => {
                error!(task_id = %task_id, "Failed to save processed TTS audio as WAV to {:?}: {}", audio_file_path, e);
                  return Err(e);
             }
        }

        // --- 创建 TxItem 并入队 ---
        let tx_item = TxItem::GeneratedVoice {
             id: Uuid::new_v4(),
             audio_data: audio_f32,
            priority: 5, // Default priority for /api/send_text items
        };
        debug!(task_id = %task_id, "Created TxItem: {:?}", tx_item);

        app_state.tx_queue.send(tx_item)
            .map_err(|e| CoreError::TxQueueSendError(format!("Failed to send to TX queue: {}", e)))?;
        info!(task_id = %task_id, "Successfully queued TxItem for transmission.");

        Ok(())
    } else {
        warn!("No active task found when trying to queue text for transmission.");
        // Also send log and status update if desired when no task is active,
        // but for now, this error is handled by the caller.
        // Consider if a global TtsStatusUpdate(Error) should be sent here.
        Err(CoreError::NoTaskRunning)
    }
}

// ----------------------------------------------------------------------------
// Utility Functions
// ----------------------------------------------------------------------------

// The get_active_tts_voice function was here.

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