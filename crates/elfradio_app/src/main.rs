#![allow(dead_code)] // Keep this for now if needed

// --- Dependency Imports ---
use std::sync::Arc;
use elfradio_config;
// 移除未使用的导入
// use elfradio_types::Config;
// 修复重复导入，只保留一个AppState
use elfradio_core::run_core_logic;
use elfradio_core::AppState;
// 删除重复的AppState导入
// use elfradio_core::state::AppState; 
use tokio::sync::{mpsc, Mutex, watch};
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};
use std::io::{self, Write};
// 修改音频处理函数导入，确保路径正确
use elfradio_core::audio_processor::audio_input_processor;
// 添加API服务器运行函数导入
use elfradio_api::run_server;
use elfradio_types::{TxItem, AudioMessage, AudioOutputSender, LogEntry, LogDirection, LogContentType, SystemServiceStatus, WebSocketMessage};
use anyhow::anyhow; // Assuming Result is not used elsewhere
// 添加数据库初始化函数导入
use elfradio_db::init_db;
// 移除其他未使用的导入
// use tracing_subscriber::fmt::format::FmtSpan;
// use std::collections::HashMap;
// use tokio::task::JoinHandle;
// use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use std::path::PathBuf;
use elfradio_types::AiError; // Import AiError directly from elfradio_types
use std::str::FromStr; // Import FromStr for Level::from_str
use chrono::Utc; // Import Utc for timestamps
// use elfradio_logging; // DELETE this line
use elfradio_types::Config as AppConfig; // Assuming AppConfig is used
use chrono;
// --- ADDED: imports for AI and AUX client creation ---
use elfradio_ai::factory::create_ai_client; // Import specifically from factory module
use elfradio_aux_client::create_aux_client; // Import from elfradio_aux_client crate
// --- ADDED: imports for network connectivity check ---
use elfradio_types::ConnectionStatus;
// 增加导入 periodic_network_connectivity_monitor 函数
use elfradio_core::network_monitor::periodic_network_connectivity_monitor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Configuration Loading ---
    // Removed the argument from the load_config call
    let config = match elfradio_config::load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            // Use a basic print here as tracing might not be initialized yet
            eprintln!("FATAL: Failed to load configuration: {}", e);
            // Consider more robust fallback or exit strategy
            // For now, creating a default config to allow logger setup
            // This might panic if default() itself fails, but it's unlikely.
            // Using default() requires AppConfig to implement Default
            AppConfig::default()
             // Alternatively, exit:
             // std::process::exit(1);
        }
    };

    // --- Logging Setup ---
    // Remove or comment out the call to the removed elfradio_logging crate
    /*
    if let Err(e) = elfradio_logging::setup_logging(&config.log_level, "elfradio_app.log") {
         eprintln!("FATAL: Failed to initialize logging: {}", e);
         std::process::exit(1);
    }
    */

    // Log after initialization is successful (This relies on the tracing setup block below)
    // tracing::info!("ElfRadio Application Starting..."); // Moved this down after setup
    // tracing::debug!("Loaded configuration: {:?}", config); // Moved this down after setup

    // --- Create channels ---
    let (tx_sender, tx_receiver) = mpsc::unbounded_channel::<TxItem>();
    let (log_entry_tx, log_entry_rx) = mpsc::unbounded_channel::<LogEntry>();
    let (_audio_input_sender, audio_input_receiver) = mpsc::unbounded_channel::<AudioMessage>();
    let (shutdown_tx, _shutdown_rx_main) = watch::channel(false);

    // 创建用于WebSocketMessage状态更新的通道
    let (status_update_tx, status_update_rx) = mpsc::unbounded_channel::<WebSocketMessage>();

    // --- Logging Initialization ---
    let config_log_level_str = &config.log_level; // Assuming config.log_level is String
    
    let final_log_level = match config_log_level_str.to_lowercase().as_str() {
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        _ => { // Handles "info", "warn", "error", empty, and any other invalid string
            let default_level = Level::INFO;
            // Check if the original string was non-empty and genuinely invalid
            // (i.e., not "info", "warn", "error", which are valid for Level::from_str)
            if !config_log_level_str.is_empty() && Level::from_str(config_log_level_str).is_err() {
                eprintln!("Warning: Invalid log level '{}' in config, defaulting to INFO", config_log_level_str);
            }
            default_level
        }
    };

    // 1. stderr writer
    let stderr_writer = std::io::stderr;

    // 2. LogEntry channel writer
    let log_entry_tx_clone = log_entry_tx.clone();
    let log_entry_make_writer = move || {
        LogEntryChannelWriter::new(log_entry_tx_clone.clone())
    };

    // 3. Combine writers
    let combined_writer = stderr_writer.and(log_entry_make_writer);

    // 4. Formatting layer
    let fmt_layer = fmt::layer()
        .with_writer(combined_writer)
        .with_ansi(true);

    // 5. Level filter
    let filter_layer = EnvFilter::builder()
        .with_default_directive(final_log_level.into()) // Use the determined level
        .from_env_lossy();

    // 6. Initialize subscriber
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
    // --- End Logging Initialization ---

    // Now it's safe to log using tracing
    tracing::info!("ElfRadio Application Starting...");
    tracing::debug!("Loaded configuration: {:?}", config); // Be careful logging sensitive info even at debug

    info!("Tracing subscriber initialized. Log level: {}", final_log_level);

    // --- Database Initialization (moved after channel creation, before full logging init) ---
    info!("Initializing Database...");
    let db_path = PathBuf::from(&config.tasks_base_directory)
        .join("elfradio_data.db")
        .to_string_lossy()
        .into_owned();
    let db_url = format!("sqlite:{}", db_path);
    let db_pool = match init_db(&db_url).await {
        Ok(pool) => {
            info!("Database pool initialized successfully.");
            pool
        }
        Err(e) => {
            eprintln!("Error: Failed to initialize database: {:?}. Exiting.", e);
            return Err(anyhow!("Database initialization failed: {}", e));
        }
    };

    // --- Initialize AI Client (Attempt) ---
    info!("Initializing AI Client (LLM)...");
    let ai_client_result = create_ai_client(&config).await; // This call remains here

    // --- Initialize Auxiliary Client (Attempt) ---
    info!("Initializing Auxiliary Client...");
    let aux_client_result = create_aux_client(&config).await; // This call remains here

    // --- Instantiate AppState ---
     info!("Initializing AppState...");
    let app_state = Arc::new(AppState::new(
        Arc::new(config.clone()),
        tx_sender,
        tx_receiver,
        Arc::new(Mutex::new(std::collections::HashMap::new())), 
        Arc::new(Mutex::new(None::<AudioOutputSender>)),
        Arc::new(Mutex::new(false)),
        shutdown_tx.clone(),         
        db_pool,
        log_entry_tx.clone(), // 为处理器传递克隆
        status_update_tx.clone()  // 为处理器传递克隆
    ));
    info!("AppState initialized.");

    // --- NOW, handle ai_client_result and store it in AppState, then send WebSocket update ---
    let llm_status_for_update: SystemServiceStatus;

    match ai_client_result { // ai_client_result was determined before AppState creation
        Ok(client_arc) => {
            info!("AI Client (LLM): Initialization successful. Determined status for LLM service: Ok.");
            llm_status_for_update = SystemServiceStatus::Ok;
            let mut ai_client_guard = app_state.ai_client.write().await; // Now app_state is in scope
            *ai_client_guard = Some(client_arc);
            debug!("Successfully stored AiClient in AppState.");
        }
        Err(ref e) => { // Use 'ref e'
            match e {
                AiError::ProviderNotSpecified => {
                    warn!("AI Client (LLM): Initialization failed because provider was not specified in 'ai_settings.provider'. Determined status for LLM service: Warning. Details: {:?}", e);
                    llm_status_for_update = SystemServiceStatus::Warning;
                }
                AiError::AuthenticationError(_) => {
                    warn!("AI Client (LLM): Initialization failed due to an authentication error (e.g., API key missing or invalid for the configured provider). Determined status for LLM service: Warning. Details: {:?}", e);
                    llm_status_for_update = SystemServiceStatus::Warning;
                }
                AiError::Config(_) => {
                    warn!("AI Client (LLM): Initialization failed due to a configuration error (e.g., missing required sub-configuration for the selected provider). Determined status for LLM service: Warning. Details: {:?}", e);
                    llm_status_for_update = SystemServiceStatus::Warning;
                }
                _ => {
                    error!("AI Client (LLM): Initialization failed due to an unexpected error. Determined status for LLM service: Error. Details: {:?}", e);
                    llm_status_for_update = SystemServiceStatus::Error;
                }
            }
            // 确保在发生错误时，app_state.ai_client 仍然是 None (已经是默认行为，因为 Ok 分支没执行)
        }
    }

    let llm_update_msg = WebSocketMessage::LlmStatusUpdate(llm_status_for_update.clone());
    if status_update_tx.send(llm_update_msg).is_err() { // status_update_tx is in scope
        error!(
            "Failed to send LlmStatusUpdate to MPSC channel. LLM Status was: {:?}",
            llm_status_for_update
        );
    } else {
        debug!(
            "LlmStatusUpdate({:?}) successfully sent to MPSC channel.",
            llm_status_for_update
        );
    }

    // --- Handle aux_client_result and store it in AppState, then send WebSocket updates ---
    let determined_aux_status: SystemServiceStatus;

    match aux_client_result { // aux_client_result was determined before AppState creation
        Ok(Some(client_arc)) => {
            info!("Auxiliary Client (for STT, TTS, Translate): Initialization successful. Determined status for these services: Ok.");
            determined_aux_status = SystemServiceStatus::Ok;
            // Store the successfully created client in AppState
            let mut aux_client_guard = app_state.aux_client.write().await;
            *aux_client_guard = Some(client_arc);
            debug!("Successfully stored AuxServiceClient in AppState.");
    }
        Ok(None) => {
            // This case means create_aux_client returned Ok(None),
            // typically because the provider was not specified or essential config (like AppKey for Aliyun TTS/STT) was missing.
            // This is considered a 'Warning' state for the services.
            warn!("Auxiliary Client (for STT, TTS, Translate): Client not created (provider not specified in 'aux_service_settings.provider', or essential sub-configuration like AppKey missing). Determined status for these services: Warning.");
            determined_aux_status = SystemServiceStatus::Warning;
            // app_state.aux_client remains None (its default)
        }
        Err(ref e) => { // Use 'ref e' to borrow the error for detailed matching
            error!("Auxiliary Client (for STT, TTS, Translate): Initialization failed. Determined status for these services: Error. Details: {:?}", e);
            // For any error during aux client creation, consider all aux services as Error.
            determined_aux_status = SystemServiceStatus::Error;
            // app_state.aux_client remains None (its default)
        }
    }

    // Send status updates for STT, TTS, and Translate services using the determined_aux_status.
    // Ensure status_update_tx is the correct, initialized MPSC sender instance.

    let services_to_update = [
        ("STT", WebSocketMessage::SttStatusUpdate(determined_aux_status.clone())),
        ("TTS", WebSocketMessage::TtsStatusUpdate(determined_aux_status.clone())),
        ("Translate", WebSocketMessage::TranslateStatusUpdate(determined_aux_status.clone())),
    ];

    for (service_name, update_msg) in services_to_update {
        if status_update_tx.send(update_msg).is_err() {
            error!(
                "Failed to send {}StatusUpdate to MPSC channel. Determined Aux Status was: {:?}",
                service_name, determined_aux_status
            );
        } else {
            debug!(
                "{}StatusUpdate({:?}) successfully sent to MPSC channel.",
                service_name, determined_aux_status
            );
        }
    }

    // --- Send Placeholder SDR Status Update (Step 5.7.7) ---
    info!("SDR: Sending placeholder status (Disconnected) on startup.");

    // Send Log Entry for SDR placeholder status
    let sdr_log_content = "SDR: Status check (placeholder) - Disconnected.";
    let sdr_log_entry = LogEntry {
        timestamp: Utc::now(),
        direction: LogDirection::Internal,
        content_type: LogContentType::Status,
        content: sdr_log_content.to_string(),
    };
    if log_entry_tx.send(sdr_log_entry).is_err() {
        error!("Failed to send SDR placeholder log entry to MPSC channel.");
    } else {
        debug!("SDR placeholder log entry sent to MPSC channel.");
    }

    // Send WebSocketMessage for SDR placeholder status
    let sdr_status_update_msg = WebSocketMessage::SdrStatusUpdate(ConnectionStatus::Disconnected);
    if status_update_tx.send(sdr_status_update_msg.clone()).is_err() { // Clone if status_update_tx is used again soon
        error!("Failed to send SdrStatusUpdate(Disconnected) to MPSC channel.");
    } else {
        debug!("SdrStatusUpdate(Disconnected) successfully sent to MPSC channel.");
    }
    // --- End Placeholder SDR Status Update ---

    // --- Start Background Tasks ---
    info!("Spawning background tasks...");

    // Core Logic Task
    let shutdown_rx_core = app_state.shutdown_tx.subscribe(); // 确保在 spawn 前定义
    let core_state_clone = app_state.clone();
    let core_handle = tokio::spawn(async move {
        run_core_logic(core_state_clone, shutdown_rx_core).await 
    });
    info!("Core Logic task spawned.");

    // Audio Input Processor Task
    let shutdown_rx_audio = app_state.shutdown_tx.subscribe(); // 确保在 spawn 前定义
    let audio_processor_state_clone = app_state.clone();
    let audio_log_tx_clone = log_entry_tx.clone(); // 为音频处理任务克隆日志发送器
    let audio_status_update_tx_clone = status_update_tx.clone(); // 为音频处理任务克隆状态更新发送器

    let audio_input_handle = tokio::spawn(async move {
        audio_input_processor(
            audio_input_receiver,
            audio_processor_state_clone,
            shutdown_rx_audio,
            audio_log_tx_clone,      // 传递克隆的日志条目发送器
            audio_status_update_tx_clone // 传递克隆的状态更新发送器
        ).await
    });
    info!("音频输入处理器任务已启动（包含日志和状态更新通道）。");

    // Periodic Network Connectivity Monitor Task
    let network_monitor_shutdown_rx = app_state.shutdown_tx.subscribe();
    let network_log_tx_clone = log_entry_tx.clone();
    let network_status_update_tx_clone = status_update_tx.clone();
    // 按照提示词移除（理论上的）下划线，确保变量名正确
    let network_monitor_handle = tokio::spawn(async move { 
        periodic_network_connectivity_monitor(
            network_status_update_tx_clone,
            network_log_tx_clone,
            network_monitor_shutdown_rx,
        ).await;
    });
    info!("Periodic network connectivity monitor task spawned.");

    // API Server Task
    info!("Starting API server...");
    let api_state_clone = app_state.clone(); 
    let api_server_handle = tokio::spawn(async move {
        run_server(api_state_clone, log_entry_rx, status_update_rx).await 
    });
    info!("API Server task spawned (with structured logging and status update channel).");

    // --- Handle Graceful Shutdown ---
    let mut shutdown_rx_main = app_state.shutdown_tx.subscribe(); // 确保此处的 shutdown_rx_main 被正确使用
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
    info!("Ctrl-C received, initiating shutdown...");
        }
        // 根据提示词，确保 shutdown_rx_main 的使用方式正确
        // 例如，如果它是一个 watch::Receiver，通常是 .changed().await
        res = shutdown_rx_main.changed() => {
            // 检查 res 是否是 Ok(())，以及 *shutdown_rx_main.borrow() 的值
            if res.is_ok() && *shutdown_rx_main.borrow() {
                 info!("Shutdown signal received from main watch channel, initiating shutdown...");
            } else if res.is_err() {
                 error!("Main shutdown watch channel closed unexpectedly or error occurred.");
            }
        }
        
        // Monitor unexpected exits of spawned tasks
        res = api_server_handle => { error!("API server task exited unexpectedly: {:?}", res); },
        res = core_handle => { error!("Core logic task exited unexpectedly: {:?}", res); },
        res = audio_input_handle => { error!("Audio input task exited unexpectedly: {:?}", res); },
        res = network_monitor_handle => { error!("Network monitor task exited unexpectedly: {:?}", res); }, 
    }

    // --- Trigger shutdown ---
    info!("Sending shutdown signal...");
    let _ = app_state.shutdown_tx.send(true); // 使用 app_state 中的 shutdown_tx

    // --- Wait for tasks to finish ---
    info!("Waiting for tasks to finish...");
    // You might add timeouts here
    // let _ = core_handle.await;
    // let _ = audio_input_handle.await;
    // let _ = api_server_handle.await; 

    info!("ElfRadio Application Exiting.");
    Ok(())
}

// --- Helper Functions (if any) ---

// Helper function (can be moved to a utility module)
fn setup_logging(_config: &elfradio_types::Config) {
    // This function is now effectively replaced by the inline logic in main
    // It can be removed or kept as a placeholder for future refactoring.
    // tracing_subscriber::fmt::init(); // Basic setup for now
}

// --- Helper Struct and Impl for Structured Logging ---

struct LogEntryChannelWriter {
    log_entry_tx: mpsc::UnboundedSender<LogEntry>,
}

impl LogEntryChannelWriter {
    fn new(log_entry_tx: mpsc::UnboundedSender<LogEntry>) -> Self {
        Self { log_entry_tx }
    }
}

impl Write for LogEntryChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let log_string = String::from_utf8_lossy(buf);

        // Create LogEntry using ONLY the fields defined in elfradio_types::LogEntry
        let entry = LogEntry {
             timestamp: chrono::Utc::now(), // Use chrono Utc for timestamp
             // Determine direction and content_type based on context if possible,
             // otherwise use defaults. Using Internal/Text as placeholder.
             direction: LogDirection::Internal, // Use an existing variant like Internal
             content_type: LogContentType::Text, // Use an existing variant like Text
             content: log_string.into_owned(), // Use the log string as content
             // DO NOT include level, message, target, task_id fields
        };

        // Send the structured LogEntry
        if self.log_entry_tx.send(entry).is_err() {
             // Log error to stderr if channel is closed
             eprintln!("Error: Failed to send structured log via MPSC channel (receiver dropped?).");
        }

        // Return Ok, pretending bytes were written to satisfy io::Write
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}