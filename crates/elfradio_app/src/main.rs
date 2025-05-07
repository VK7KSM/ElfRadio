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
use tokio::sync::{mpsc, Mutex, RwLock, watch};
use tracing::{error, info, warn, Level};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};
use std::io::{self, Write};
// 修改音频处理函数导入，确保路径正确
use elfradio_core::audio_processor::audio_input_processor;
// 添加API服务器运行函数导入
use elfradio_api::run_server;
use elfradio_types::{TxItem, AudioMessage, ClientMap, AudioOutputSender, LogEntry, LogDirection, LogContentType};
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

    // --- Logging Initialization ---
    let config_log_level_str = &config.log_level;
    let log_level = Level::from_str(config_log_level_str)
        .unwrap_or_else(|_| {
            eprintln!("Warning: Invalid log level '{}' in config, defaulting to INFO", config_log_level_str);
            Level::INFO
        });

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
        .with_default_directive(log_level.into())
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

    info!("Tracing subscriber initialized. Log level: {}", log_level);

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

    // --- Initialize AI Client (moved after channel/db creation) ---
    info!("Initializing AI Client...");
    let ai_client_result = create_ai_client(&config).await; // Pass the whole config

    // --- Initialize Auxiliary Client ---
    info!("Initializing Auxiliary Client...");
    // Call the factory function from elfradio_aux_client
    let aux_client_result = create_aux_client(&config).await; // Use .await as it's async

    let aux_client_for_state = match aux_client_result {
        Ok(Some(client)) => {
            info!("Auxiliary Client initialized successfully.");
            Some(client)
        }
        Ok(None) => {
            warn!("Auxiliary Client not configured or failed to initialize. Aux features unavailable.");
            None
        }
        Err(e) => {
            error!("Unexpected error during auxiliary client creation: {}. Aux features unavailable.", e);
            None
        }
    };

    // --- Instantiate AppState (moved after channel/db/ai creation) ---
    info!("Initializing AppState...");
    let app_state = Arc::new(AppState::new(
        Arc::new(config.clone()),
        tx_sender,
        tx_receiver,
        Arc::new(Mutex::new(std::collections::HashMap::new())) as ClientMap,
        Arc::new(Mutex::new(None::<AudioOutputSender>)),
        Arc::new(Mutex::new(false)),
        shutdown_tx,
        db_pool, // Pass the initialized db_pool
    ));

    // Handle AI Client result AFTER AppState exists
    if let Ok(client) = ai_client_result {
        info!("AI Client created successfully, storing in AppState.");
        let mut ai_client_guard = app_state.ai_client.write().await;
        *ai_client_guard = Some(client);
    } else if let Err(AiError::ProviderNotSpecified) = ai_client_result {
        warn!("AI Provider not specified in config. AI features unavailable until configured.");
    } else if let Err(e) = ai_client_result {
        error!("Failed to create AI client during startup: {:?}. AI features unavailable.", e);
        // 不因此退出程序，只记录错误并继续
    }

    // Set the aux_client in AppState:
    if let Some(client) = aux_client_for_state {
        let mut aux_guard = app_state.aux_client.write().await;
        *aux_guard = Some(client);
        info!("Auxiliary Client stored in AppState.");
    }

    // Now we can safely use info!, warn!, error! macros which will go to both targets
    info!("AppState initialized."); // Log this again now that tracing is fully set up

    // --- Get shutdown receivers for tasks ---
    let shutdown_rx_audio = app_state.shutdown_tx.subscribe();
    let shutdown_rx_core = app_state.shutdown_tx.subscribe();

    // --- Start Background Tasks ---
    info!("Spawning background tasks...");
    let core_state = app_state.clone();
    let core_handle = tokio::spawn(async move {
        run_core_logic(core_state, shutdown_rx_core).await
    });
    info!("Core Logic task spawned.");

    let audio_processor_state = app_state.clone();
    let audio_input_handle = tokio::spawn(async move {
        audio_input_processor(
            audio_input_receiver,
            audio_processor_state,
            shutdown_rx_audio
        ).await
    });
    info!("Audio Input Processor task spawned.");

    // Start API server task
    info!("Starting API server...");
    let api_state = app_state.clone();
    let api_server_handle = tokio::spawn(async move {
        run_server(api_state, log_entry_rx).await
    });
    info!("API Server task spawned (with structured logging).");

    // --- Handle Graceful Shutdown ---
    let mut shutdown_rx_main = app_state.shutdown_tx.subscribe();
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
    info!("Ctrl-C received, initiating shutdown...");
        }
        _ = shutdown_rx_main.changed() => {
            info!("Shutdown signal received from watch channel, initiating shutdown...");
}
        // Optionally wait for task completion or error
        res = api_server_handle => { error!("API server task exited unexpectedly: {:?}", res); },
        res = core_handle => { error!("Core logic task exited unexpectedly: {:?}", res); },
        res = audio_input_handle => { error!("Audio input task exited unexpectedly: {:?}", res); },
    }

    // --- Trigger shutdown ---
    info!("Sending shutdown signal...");
    let _ = app_state.shutdown_tx.send(true);

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