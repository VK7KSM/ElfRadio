mod error; // Add this line to declare the error module

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path as AxumPath,
        State,
        Json,
    },
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
    debug_handler,
};
use elfradio_core::AppState;
use elfradio_core::CoreError; // 添加此行导入 CoreError
use futures_util::{
    sink::SinkExt, 
    stream::{SplitSink, SplitStream, StreamExt} // 添加 SplitSink, SplitStream
};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::mpsc; // 移除 Mutex，未使用
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};
use std::io::{Cursor, ErrorKind, Read, Write};
use serde::Deserialize;
use elfradio_core::tx_processor; // Import the module
use elfradio_types::TaskMode; // Import Log types and TaskInfo
use std::path::PathBuf;
use serde_json::json; // 确保 json 宏已导入
use elfradio_types::{Config, LogEntry, WebSocketMessage, FrontendConfig}; // Import necessary types: LogEntry, TaskMode, WebSocketMessage, FrontendConfig
use elfradio_types::UpdateConfigRequest; // Import UpdateConfigRequest
use elfradio_config::{save_user_config_values, ConfigError as ElfConfigError}; // Import save function and ConfigError
use crate::error::ApiError; // Use local ApiError
use serde_json::Value as JsonValue; // Ensure this import is present

/// API 服务器的主入口函数。
///
/// 接收共享的应用状态和结构化日志条目接收器。
pub async fn run_server(
    app_state: Arc<AppState>,
    mut log_entry_rx: mpsc::UnboundedReceiver<LogEntry>, // Changed parameter type to LogEntry receiver
) -> Result<(), anyhow::Error> {
    // --- Start Log Broadcast Task (Singleton) ---
    match app_state.log_broadcast_task_handle.get_or_try_init(|| async {
        info!("Initializing WebSocket structured log broadcast task...");
        let clients = Arc::clone(&app_state.clients);
        
        Ok::<tokio::task::JoinHandle<()>, tokio::sync::AcquireError>(tokio::spawn(async move {
            // Loop while the LogEntry receiver is active
            while let Some(log_entry) = log_entry_rx.recv().await { // Receive LogEntry struct
                let clients_guard = clients.lock().await;
                if clients_guard.is_empty() {
                    continue; // Skip if no clients are connected
                }

                // --- Create WebSocketMessage directly from received LogEntry ---
                let ws_message = WebSocketMessage::Log(log_entry); // Use the received LogEntry

                // --- Serialize to JSON ---
                match serde_json::to_string(&ws_message) {
                    Ok(json_string) => {
                        debug!(num_clients = clients_guard.len(), "Broadcasting structured log message (JSON)");
                        let ws_msg = Message::Text(json_string); // Send JSON string

                // Iterate over all connected client senders
                for tx in clients_guard.values() {
                    // Send the log message. If send fails, the client likely disconnected.
                            if tx.send(Ok(ws_msg.clone())).is_err() {
                        // Error sending: client task likely ended.
                        // The cleanup logic in handle_socket should remove the client soon.
                    }
                }
            }
                    Err(e) => {
                        error!("Failed to serialize LogEntry message to JSON: {:?}", e);
                        // Consider logging the failed LogEntry struct itself
                    }
                }
            }
            info!("WebSocket structured log broadcast task finished.");
        }))
    }).await {
        Ok(_) => {
            info!("Structured log broadcast task initialization successful.");
        }
        Err(e) => {
            error!("Failed to initialize structured log broadcast task: {}", e);
            return Err(anyhow::anyhow!("Failed to start structured log broadcast task: {}", e));
        }
    }

    // 创建 CORS layer - 允许所有来源 (开发时方便，生产环境应更严格)
    let cors = CorsLayer::new()
        .allow_origin(Any) //允许任何来源
        .allow_methods(Any) //允许任何 HTTP 方法
        .allow_headers(Any); //允许任何 HTTP 头

    // 创建 Axum 路由器
    // 将 app_state 和 ws_log_rx_singleton 一起作为共享状态传递
    let app = Router::new()
        .route("/api/health", get(health_check_handler))
        .route("/ws", get(websocket_handler)) // WebSocket 路由
        .route("/api/tasks/:task_id/export", get(export_task_data_handler))
        .route("/api/send_text", post(send_text_handler)) // Add the new route
        .route("/api/start_task", post(start_task_handler)) // 添加 /api/start_task 路由
        .route("/api/stop_task", post(stop_task_handler)) // 添加 /api/stop_task 路由
        .route("/api/config", get(get_config_handler))
        .route("/api/config/update", post(update_config_handler)) // Add the new route for updating configuration
        .with_state(app_state) // Pass only AppState
        .layer(cors); // 应用 CORS 中间件

    // 定义监听地址
    let listen_address = "0.0.0.0"; // 默认监听所有接口
    let listen_port = 5900; // 使用固定端口 5900

    let addr = SocketAddr::from((listen_address.parse::<std::net::IpAddr>().unwrap_or_else(|_| 
        "0.0.0.0".parse().unwrap()), listen_port));
    info!("API 服务器正在监听于 {}", addr);

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("API Server listening on http://{}", listener.local_addr()?); // 添加日志记录实际地址
    axum::serve(listener, app.into_make_service())
        .await?; // 服务器会一直运行直到出错或被关闭

    Ok(())
}

/// 处理 `/api/health` 路由的简单处理器。
async fn health_check_handler() -> &'static str {
    "OK"
}

/// 处理 `/ws` 路由，升级连接到 WebSocket。
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>, // Get AppState from shared state
) -> Response {
    info!("WebSocket 客户端尝试连接...");
    // Pass AppState clone to handle_socket
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// 处理单个 WebSocket 连接。
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    // --- Add this debug log ---
    tracing::debug!("WebSocket connection upgrade successful, handling socket...");
    // --- End of added log ---

    let client_id = Uuid::new_v4();
    info!(%client_id, "WebSocket 客户端已连接！");

    // 1. Create a channel specifically for this client connection
    let (client_tx, client_rx) =
        mpsc::unbounded_channel::<Result<Message, axum::Error>>();

    // 2. Add the client's sender channel to the shared map
    {
        // 先锁定 clients 并插入
        let mut clients_guard = state.clients.lock().await;
        clients_guard.insert(client_id, client_tx);
        
        // 获取数量后再输出日志
        let count = clients_guard.len();
        info!(%client_id, count, "客户端已添加到映射中");
        // clients_guard 在此作用域结束时自动释放
    }

    // 3. Split the WebSocket into a sender and receiver
    let (sink, stream) = socket.split();

    // --- Spawn Task to Forward Messages TO the Client ---
    // This task listens on the client-specific channel (client_rx)
    // and sends received messages to the WebSocket sink.
    let send_task = tokio::spawn(forward_to_client(client_rx, sink, client_id));

    // --- Spawn Task to Handle Messages FROM the Client ---
    // This task listens on the WebSocket stream for incoming messages.
    let receive_task = tokio::spawn(handle_incoming(stream, client_id, Arc::clone(&state)));


    // Keep handle_socket alive until either task finishes (indicating disconnection)
    // This allows cleanup logic to run reliably afterwards.
    tokio::select! {
        _ = send_task => { info!(%client_id, "发送任务结束"); },
        _ = receive_task => { info!(%client_id, "接收任务结束"); },
    }


    // --- Cleanup ---
    // This code runs when either the send or receive task completes.
    info!(%client_id, "客户端连接断开，正在清理...");
    {
        // Lock the map and remove the client's sender channel
        let mut clients_guard = state.clients.lock().await;
        if clients_guard.remove(&client_id).is_some() {
            info!(%client_id, count = clients_guard.len(), "客户端已从映射中移除");
        } else {
            warn!(%client_id, "尝试移除客户端，但未在映射中找到");
        }
        // Mutex guard is dropped here
    }
    info!(%client_id, "客户端清理完成。");
}

/// Task to forward messages from the client's MPSC channel to the WebSocket sink.
async fn forward_to_client(
    mut client_rx: mpsc::UnboundedReceiver<Result<Message, axum::Error>>,
    mut sink: SplitSink<WebSocket, Message>,
    client_id: Uuid,
) {
     while let Some(result) = client_rx.recv().await {
         match result {
             Ok(msg) => {
                 if sink.send(msg).await.is_err() {
                     warn!(%client_id, "发送消息到 WebSocket sink 失败，连接可能已关闭。");
                     // Error sending means the sink is closed, break the loop.
                     break;
                 }
             }
             Err(e) => {
                 // This would likely mean an error occurred elsewhere trying to send *to* this client.
                 error!(%client_id, "从客户端通道接收到错误: {}", e);
                 // Optionally send an error message to the client? Depends on requirements.
                 // Let's break for now if we receive an error on the channel.
                 break;
             }
         }
     }
     info!(%client_id, "客户端 MPSC 通道关闭或发送失败，转发任务结束。");
     // Ensure the sink is closed gracefully if possible
     let _ = sink.close().await;
}

/// Task to handle incoming messages from the WebSocket stream.
async fn handle_incoming(
    mut stream: SplitStream<WebSocket>,
    client_id: Uuid,
    _state: Arc<AppState>, // 添加下划线前缀
) {
    while let Some(result) = stream.next().await {
        match result {
            Ok(msg) => {
                match msg {
                    Message::Text(text) => {
                        info!(%client_id, "收到来自客户端的文本消息: {}", text);
                        // TODO: 在这里处理客户端命令, 例如发送到 state.tx_queue
                        // Example: if text == "SEND_SOMETHING" { state.tx_queue.send(...)? }
                    }
                    Message::Binary(bin) => {
                        info!(%client_id, "收到来自客户端的二进制消息: {} bytes", bin.len());
                        // Handle binary data if necessary
                    }
                    Message::Ping(ping) => {
                        debug!(%client_id, "收到 Ping: {:?}", ping);
                        // Axum handles Pong automatically unless you turn it off
                    }
                    Message::Pong(pong) => {
                        debug!(%client_id, "收到 Pong: {:?}", pong);
                    }
                    Message::Close(close) => {
                        info!(%client_id, "收到客户端的关闭帧: {:?}", close);
                        // Client initiated close, break the loop.
                        break;
                    }
                }
            }
            Err(e) => {
                warn!(%client_id, "从 WebSocket 流读取时出错: {}", e);
                // Error reading likely means connection closed, break the loop.
                break;
            }
        }
    }
    info!(%client_id, "WebSocket 流结束或出错，接收任务结束。");
}

// Define a Result type alias for API handlers
type ApiResult<T> = Result<T, ApiError>;

// --- New Export Handler ---
async fn export_task_data_handler(
    AxumPath(task_id): AxumPath<Uuid>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Response> { // Use ApiResult for return type
    info!(%task_id, "Received request to export task data.");

    // 1. Get Task Directory Path
    // Access config directly as it's Arc<Config>
    let task_dir = state
        .config
        .tasks_base_directory
        .join(task_id.to_string());
    debug!(%task_id, "Target task directory: {:?}", task_dir);

    // 2. Check Directory Existence (Async)
    if let Err(e) = tokio::fs::metadata(&task_dir).await {
        match e.kind() {
            ErrorKind::NotFound => {
                warn!(%task_id, "Task directory not found: {:?}", task_dir);
                return Err(ApiError::TaskNotFound(task_id));
            }
            _ => {
                error!(%task_id, "Error accessing task directory metadata {:?}: {}", task_dir, e);
                // Map other IO errors during metadata check
                return Err(ApiError::IoError(e));
            }
        }
    }
    info!(%task_id, "Task directory found.");

    // 3. Create ZIP Archive (in blocking task)
    let zip_data_result: Result<Vec<u8>, ApiError> = tokio::task::spawn_blocking(move || {
        // Define the specific file we want to zip for now
        let target_filename = "events.jsonl";
        let file_path = task_dir.join(target_filename);
        info!(%task_id, "Attempting to zip file: {:?}", file_path);

        // 2. Check if target file exists
        if !file_path.exists() {
            warn!(%task_id, path = %file_path.display(), "events.jsonl file not found, returning empty ZIP data.");
             // Return Ok with an empty Vec<u8> and the explicit error type ZipError
             return Ok::<Vec<u8>, zip::result::ZipError>(Vec::new()); // Successfully created an empty archive
        }

        let mut buffer: Vec<u8> = Vec::new();
        { // Scope for cursor and zip_writer borrowing buffer
            let cursor = Cursor::new(&mut buffer);
            let mut zip_writer = ZipWriter::new(cursor);

            // Use Stored for no compression, Deflated for compression
            let options = FileOptions::default()
                .compression_method(CompressionMethod::Deflated) // Use compression
                .unix_permissions(0o644); // Set basic permissions

            // Add the events.jsonl file
            // Map IO errors within the closure to ZipError::Io
            zip_writer
                .start_file(target_filename, options)
                .map_err(zip::result::ZipError::from)?; // Map ZipResult error

            let mut file_content = Vec::new();
            let mut file = std::fs::File::open(&file_path)
                           .map_err(zip::result::ZipError::Io)?; // Corrected: Remove Arc::new wrapper

            file.read_to_end(&mut file_content)
                .map_err(zip::result::ZipError::Io)?; // Corrected: Remove Arc::new wrapper

            zip_writer
                .write_all(&file_content)
                .map_err(zip::result::ZipError::Io)?; // Corrected: Remove Arc::new wrapper

             // Add more files here later if needed by iterating task_dir
             // e.g., using walkdir crate for recursive zipping

            zip_writer.finish().map_err(zip::result::ZipError::from)?; // Finish archive
        } // zip_writer and cursor are dropped here, releasing the borrow on buffer

        info!(%task_id, "Successfully created ZIP archive in memory ({} bytes)", buffer.len());
        Ok(buffer) // Return the buffer containing ZIP data
    })
    .await
    // Handle JoinError from spawn_blocking
    .map_err(|e| ApiError::InternalServerError(format!("ZIP creation task failed: {}", e)))?
    // The inner Result<Vec<u8>, zip::result::ZipError> is now handled:
    .map_err(ApiError::from); // Use From trait implementation

    // 4. Handle ZIP Result
    let data = zip_data_result?; // Propagate ApiError if zipping failed

    // Check if the returned data is empty (which we decided means the log file wasn't found)
     if data.is_empty() {
         warn!(%task_id, "events.jsonl not found, returning empty ZIP response.");
         // Optionally return 404 here instead of empty zip?
         // For now, return success with empty zip as implemented in blocking task.
     }


    // 5. Build Response
    let filename = format!("elfradio_task_{}.zip", task_id);
    info!(%task_id, "Sending ZIP file: {}", filename);

    Ok((
        // Use axum::http::header constants
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename), // Borrow filename
            ),
        ],
        data, // The Vec<u8> containing ZIP data
    )
        .into_response())
}

// Define the request body structure
#[derive(Deserialize, Debug)]
pub struct SendTextRequest {
    pub text: String,
}

// API Handler function for sending text
async fn send_text_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SendTextRequest>,
) -> Result<StatusCode, ApiError> {
    info!("Received request to send text: '{}'", payload.text);

    // --- TODO: Refactor AppState to store active task info (ID and directory) ---
    // The current AppState structure does not seem to have an 'active_task' field.
    // The logic below for getting task_dir_opt and task_id_opt is commented out.
    /*
    let (task_dir_opt, task_id_opt): (Option<PathBuf>, Option<String>) = {
        // This requires AppState to have `pub active_task: Mutex<Option<TaskInfo>>` field
        let active_task_guard = state.active_task.lock().await; // <-- This line causes E0609
        let task_info_opt = active_task_guard.as_ref();
        let dir = task_info_opt.map(|info| info.task_dir.clone());
        let id = task_info_opt.map(|info| info.id.clone());
        (dir, id)
    };
    */
    // --- Temporary workaround: Assume no task directory available for now ---
    let _task_dir_opt: Option<PathBuf> = None; // Placeholder
    let _task_id_opt: Option<String> = None;   // Placeholder


    // --- Call Core Logic (Queue Text for Transmission) ---
    let app_state_clone = state.clone();
    let text_to_send = payload.text.clone();

    match tx_processor::queue_text_for_transmission(
        app_state_clone,
        text_to_send,
    ).await {
        Ok(_) => {
            info!("Successfully queued text for transmission via WebSocket.");
            Ok(StatusCode::ACCEPTED)
        }
        Err(CoreError::AiNotConfigured) => {
            warn!("API request failed: AI not configured.");
            Err(ApiError::AiNotConfigured) // 映射到专门的 ApiError
        }
        Err(CoreError::TaskAlreadyRunning) => {
            warn!("API request failed: Task already running.");
            Err(ApiError::TaskAlreadyRunning) // 映射已存在的错误
        }
        Err(e) => {
            error!("Error queuing text for transmission: {:?}", e);
            Err(ApiError::InternalServerError(format!("Error queuing text: {}", e)))
        }
    }
}

// --- 新添加的请求结构体 ---
#[derive(Deserialize, Debug)]
pub struct StartTaskRequest {
    pub mode: TaskMode,
}

/// 处理 `/api/start_task` POST 请求的处理器。
#[debug_handler]
async fn start_task_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StartTaskRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    info!("Received request to start task: {:?}", payload);
    match elfradio_core::start_task(state, payload.mode).await {
        Ok(task_id) => {
            info!("Task started successfully with ID: {}", task_id);
            Ok((StatusCode::OK, Json(json!({ "task_id": task_id.to_string() }))))
        }
        Err(CoreError::TaskAlreadyRunning) => {
            error!("Failed to start task: Task already running.");
            Err(ApiError::TaskAlreadyRunning)
        }
        Err(CoreError::AiNotConfigured) => {
            warn!("API request failed: AI not configured.");
            Err(ApiError::AiNotConfigured) // 映射到专门的 ApiError
        }
        Err(e) => {
            error!("Failed to start task due to internal error: {:?}", e);
            Err(ApiError::InternalServerError(format!("Failed to start task: {}", e)))
        }
    }
}

/// 处理 `/api/stop_task` POST 请求的处理器。
async fn stop_task_handler(
    State(state): State<Arc<AppState>>, // 使用 State 提取共享状态
) -> Result<StatusCode, ApiError> { // 返回 Result<StatusCode, ApiError>
    info!("Received request to stop task.");
    // 调用 elfradio_core 中的 stop_task 函数
    match elfradio_core::stop_task(state).await {
        Ok(()) => {
            info!("Task stopped successfully via API.");
            Ok(StatusCode::OK) // 成功停止，返回 200 OK
        }
        Err(CoreError::NoTaskRunning) => {
            // 如果没有任务在运行，也视为成功（幂等性）
            warn!("Stop task request received, but no task was running.");
            Ok(StatusCode::OK) // 返回 200 OK
        }
        Err(e) => {
            // 其他 CoreError 视为内部服务器错误
            error!("Failed to stop task due to internal error: {:?}", e);
            Err(ApiError::InternalServerError(format!("Failed to stop task: {}", e)))
        }
    }
}

/// Handles GET requests to retrieve non-sensitive configuration for the frontend.
pub async fn get_config_handler(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<FrontendConfig>, ApiError> {
    info!("Received request to get frontend configuration.");
    // AppState.config field is Arc<Config>, dereferencing it gives &Config
    let config_ref: &Config = &app_state.config; // This line should now compile
    // Convert the full config to the safe frontend version using the From impl
    let frontend_config = FrontendConfig::from(config_ref);
    debug!("Returning frontend configuration: {:?}", frontend_config);
    Ok(Json(frontend_config))
}

/// Handles POST requests to update configuration values.
pub async fn update_config_handler(
    State(_app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateConfigRequest>
) -> Result<StatusCode, ApiError> {
    info!("Received request to update configuration with {} keys.", payload.updates.len());
    debug!("Payload: {:?}", payload);

    // Use the imported alias JsonValue or explicit type serde_json::Value
    let values_to_save = JsonValue::Object(payload.updates.into_iter().collect());
    // Alternative:
    // let values_to_save = serde_json::Value::Object(payload.updates.into_iter().collect());

    match save_user_config_values(values_to_save) {
        Ok(_) => {
            info!("Configuration updated successfully via API.");
            Ok(StatusCode::OK)
        }
        Err(config_err) => {
            error!("Failed to save configuration via API: {}", config_err);
            match config_err {
                ElfConfigError::IoError(_) => Err(ApiError::InternalServerError(format!(
                    "Failed to write configuration file: {}",
                    config_err
                ))),
                ElfConfigError::Config(rs_err) => {
                     if rs_err.to_string().contains("Invalid user TOML format") {
                          // Ensure ApiError::BadRequest exists in error.rs
                          Err(ApiError::BadRequest(format!( // Using BadRequest variant
                               "Failed to save: Invalid existing config format. Please check/reset config file. Error: {}",
                               rs_err
                          )))
                     } else if rs_err.to_string().contains("Expected a JSON object") {
                           // Ensure ApiError::BadRequest exists in error.rs
                           Err(ApiError::BadRequest(format!( // Using BadRequest variant
                               "Internal error creating save data: {}",
                               rs_err
                          )))
                     }
                     else {
                           Err(ApiError::InternalServerError(format!(
                                "Failed to process configuration update: {}",
                                rs_err
                           )))
                     }
                },
                ElfConfigError::DirectoryError => Err(ApiError::InternalServerError(format!(
                    "Failed to access configuration directory: {}",
                    config_err
                ))),
            }
        }
    }
}
