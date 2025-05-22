mod error; // Add this line to declare the error module
mod test_handlers; // ADD THIS LINE to declare the new handlers module

/// Request body for the temporary translation test endpoint.
#[derive(Deserialize, Debug)]
pub struct TestTranslateRequest { // Made pub for potential use in handler module
    pub text: String,
    pub target_language: String,
    pub source_language: Option<String>,
    // Optional: Add a field to specify provider if needed for more advanced testing later
    // pub provider: Option<String>,
}

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
use zip::{CompressionMethod, ZipWriter};
use std::io::{Cursor, ErrorKind, Read, Write};
use serde::Deserialize;
use elfradio_core::tx_processor; // Import the module
use elfradio_types::TaskMode; // Import Log types and TaskInfo
use serde_json::json; // 确保 json 宏已导入
use elfradio_types::{Config, LogEntry, WebSocketMessage, FrontendConfig, ConnectionStatus}; // Import necessary types: LogEntry, TaskMode, WebSocketMessage, FrontendConfig, ConnectionStatus
use elfradio_types::UpdateConfigRequest; // Import UpdateConfigRequest
use elfradio_types::TestLlmRequest; // Import the request struct from elfradio_types
use elfradio_config::{save_user_config_values, ConfigError as ElfConfigError}; // Import save function and ConfigError
use crate::error::ApiError; // Use local ApiError
use serde_json::Value as JsonValue; // Ensure this import is present
use elfradio_types::{AiError as ElfAiError};
use elfradio_ai::AiClient; // CRITICAL: Import AiClient trait from elfradio_ai
 // ADD THESE LINES for the new request types from elfradio_types
use elfradio_types::{
    LogDirection, LogContentType, SystemServiceStatus
};
use chrono::Utc;

/// API 服务器的主入口函数。
///
/// 接收共享的应用状态和结构化日志条目接收器。
pub async fn run_server(
    app_state: Arc<AppState>,
    mut log_entry_rx: mpsc::UnboundedReceiver<LogEntry>, // Changed parameter type to LogEntry receiver
    mut status_update_rx: mpsc::UnboundedReceiver<WebSocketMessage>, // 新增的状态更新通道
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
                        // Replace tracing::debug! with eprintln! to break the feedback loop
                        // We can still include useful info like the number of clients.
                        // Note: println! or eprintln! will go directly to the console where the backend is running.
                        // If you still want it to be level-filterable by the main tracing setup (but not go to MPSC),
                        // that would require more complex subscriber filtering. For now, direct print is simplest.
                        if clients_guard.len() > 0 { // Only print if there are clients to avoid spamming when idle
                            eprintln!(
                                "[LOG_BROADCAST_DEBUG] Broadcasting structured log to {} client(s). First ~50 chars: {:.50}",
                                clients_guard.len(),
                                json_string
                            );
                        }
                        
                        let ws_msg = axum::extract::ws::Message::Text(json_string.into());

                for tx in clients_guard.values() {
                            if tx.send(Ok(ws_msg.clone())).is_err() {
                                // Error sending, client task likely ended.
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

    info!("正在初始化 WebSocket 系统状态更新广播任务...");
    let status_clients_ref = Arc::clone(&app_state.clients); // 为新任务克隆 Arc
    let _status_broadcast_handle = tokio::spawn(async move {
        while let Some(ws_message) = status_update_rx.recv().await {
            let clients_guard = status_clients_ref.lock().await;
            if clients_guard.is_empty() {
                // 当收到消息但没有客户端连接时记录日志
                eprintln!("[STATUS_BROADCAST_DEBUG] Received WebSocketMessage (type: {:?}) but no clients connected, skipping broadcast.", message_type_for_debug(&ws_message));
                continue;
            }

            match serde_json::to_string(&ws_message) {
                Ok(json_string) => {
                    // 此 eprintln! 如果序列化成功且存在客户端，则应始终执行。
                    eprintln!(
                        "[STATUS_BROADCAST_DEBUG] Broadcasting WebSocketMessage (type: {:?}) to {} client(s). JSON: {:.100}", // 记录部分 JSON
                        message_type_for_debug(&ws_message),
                        clients_guard.len(),
                        json_string // 记录实际发送的 JSON 字符串（或片段）
                    );

                    let axum_ws_msg = axum::extract::ws::Message::Text(json_string.into()); // 使用 .into()
                    for (_client_id, sender) in clients_guard.iter() {
                        if sender.send(Ok(axum_ws_msg.clone())).is_err() {
                            // 客户端已断开连接，将由 handle_socket 清理
                            // 如果需要，可在此处考虑添加 trace 日志:
                            // tracing::trace!("Failed to send status update to client_id: {}, likely disconnected.", _client_id);
                        }
                    }
                }
                Err(e) => {
                    // 这个错误日志很重要
                    error!("Failed to serialize WebSocketMessage for status update: {:?}. Original message type: {:?}", e, message_type_for_debug(&ws_message));
                }
            }
        }
        info!("WebSocket 系统状态更新广播任务已完成。");
    });
    info!("系统状态更新广播任务已启动。");

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
        .route("/api/tasks/{task_id}/export", get(export_task_data_handler))
        .route("/api/send_text", post(send_text_handler)) // Add the new route
        .route("/api/start_task", post(start_task_handler)) // 添加 /api/start_task 路由
        .route("/api/stop_task", post(stop_task_handler)) // 添加 /api/stop_task 路由
        .route("/api/config", get(get_config_handler))
        .route("/api/config/update", post(update_config_handler)) // Add the new route for updating configuration
        .route("/api/test/translate", post(test_translate_handler)) // Add the new route for testing translation
        .route("/api/test/llm", post(test_llm_handler)) // Add the new LLM test route
        .route("/api/test/tts", post(test_handlers::test_tts_handler)) // ADD THESE TWO NEW ROUTES for TTS and STT testing
        .route("/api/test/stt", post(test_handlers::test_stt_handler))
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
        clients_guard.insert(client_id, client_tx.clone());
        
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
    let send_task_handle = tokio::spawn(forward_to_client(client_rx, sink, client_id)); // 保存 handle

    // --- Spawn Task to Handle Messages FROM the Client ---
    // This task listens on the WebSocket stream for incoming messages.
    let receive_task_handle = tokio::spawn(handle_incoming(stream, client_id, Arc::clone(&state))); // 保存 handle


    // 新增: 连接成功后立即发送用户UUID给客户端
    if let Some(uuid) = &state.config.user_uuid {
        let uuid_message = WebSocketMessage::UserUuidUpdate(Some(uuid.clone()));
        match serde_json::to_string(&uuid_message) {
            Ok(json_string) => {
                let ws_msg = Message::Text(json_string.into());
                // 直接发送到这个特定客户端
                if client_tx.send(Ok(ws_msg)).is_err() {
                    warn!(%client_id, "发送用户 UUID 到客户端失败");
                } else {
                    debug!(%client_id, uuid=%uuid, "已发送用户 UUID 到客户端");
                }
            }
            Err(e) => error!(%client_id, "序列化用户 UUID 消息失败: {:?}", e),
        }
    } else {
        // 如果 state.config.user_uuid 是 None，也要通知客户端
        let uuid_message = WebSocketMessage::UserUuidUpdate(None);
        match serde_json::to_string(&uuid_message) {
            Ok(json_string) => {
                let ws_msg = Message::Text(json_string.into());
                if client_tx.send(Ok(ws_msg)).is_err() {
                    warn!(%client_id, "发送空用户 UUID 到客户端失败");
                } else {
                    debug!(%client_id, "已发送空用户 UUID 到客户端");
                }
            }
            Err(e) => error!(%client_id, "序列化空用户 UUID 消息失败: {:?}", e),
        }
    }

    // --- ADD NEW LOGIC HERE to send other initial system statuses ---
    info!(%client_id, "Sending initial system statuses to newly connected client.");

    // Helper closure to send a WebSocketMessage
    let send_ws_msg = |tx: &mpsc::UnboundedSender<Result<Message, axum::Error>>, msg: WebSocketMessage, service_name: &str| {
        match serde_json::to_string(&msg) {
            Ok(json_string) => {
                if tx.send(Ok(Message::Text(json_string.into()))).is_err() {
                    warn!(%client_id, "Failed to send initial {} status to client.", service_name);
                } else {
                    // 对于枚举的 payload，直接使用 Debug trait
                    debug!(%client_id, "Sent initial {} status to client: {:?}", service_name, msg);
                }
            }
            Err(e) => {
                error!(%client_id, "Failed to serialize initial {} status message: {:?}", service_name, e);
            }
        }
    };
    
    // LLM Status
    let llm_initial_status = if state.ai_client.read().await.is_some() { SystemServiceStatus::Ok } else { SystemServiceStatus::Warning };
    send_ws_msg(&client_tx, WebSocketMessage::LlmStatusUpdate(llm_initial_status), "LLM");

    // STT, TTS, Translate Status
    let aux_initial_status = if state.aux_client.read().await.is_some() { SystemServiceStatus::Ok } else { SystemServiceStatus::Warning };
    send_ws_msg(&client_tx, WebSocketMessage::SttStatusUpdate(aux_initial_status.clone()), "STT");
    send_ws_msg(&client_tx, WebSocketMessage::TtsStatusUpdate(aux_initial_status.clone()), "TTS");
    send_ws_msg(&client_tx, WebSocketMessage::TranslateStatusUpdate(aux_initial_status), "Translate");

    // SDR Status
    send_ws_msg(&client_tx, WebSocketMessage::SdrStatusUpdate(ConnectionStatus::Disconnected), "SDR");

    // Network Connectivity Status
    send_ws_msg(&client_tx, WebSocketMessage::NetworkConnectivityUpdate(ConnectionStatus::Checking), "Network");
    
    // Radio Status
    send_ws_msg(&client_tx, WebSocketMessage::RadioStatusUpdate(ConnectionStatus::Unknown), "Radio");
    // --- END NEW LOGIC ---


    // Keep handle_socket alive until either task finishes (indicating disconnection)
    // This allows cleanup logic to run reliably afterwards.
    tokio::select! {
        res_send = send_task_handle => { 
            info!(%client_id, "发送任务结束"); 
            if let Err(e) = res_send {
                error!(%client_id, "发送任务 JoinError: {:?}", e);
            }
        },
        res_receive = receive_task_handle => { 
            info!(%client_id, "接收任务结束"); 
            if let Err(e) = res_receive {
                error!(%client_id, "接收任务 JoinError: {:?}", e);
            }
        },
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
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .compression_level(Some(6)); // Optional: Set compression level (0-9)

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
    info!("收到发送文本请求: '{}'", payload.text);

    // 获取要发送的文本
    let text_to_send = payload.text.clone();
    
    // 先从原始 state 获取发送器的引用，避免在移动 app_state_clone 后再尝试借用它
    let log_tx_ref = &state.log_entry_tx_for_handlers;
    let status_tx_ref = &state.status_update_tx_for_handlers;
    
    // 克隆 Arc<AppState> 将会被移动到函数中
    let app_state_for_core = state.clone();

    match tx_processor::queue_text_for_transmission(
        app_state_for_core, // 此 Arc 实例将被移动
        text_to_send,
        log_tx_ref,         // 传递对 log_entry_tx_for_handlers 的引用
        status_tx_ref       // 传递对 status_update_tx_for_handlers 的引用
    ).await {
        Ok(_) => {
            info!("成功通过 API 调用将文本排队进行传输。");
            Ok(StatusCode::ACCEPTED)
        }
        Err(CoreError::AiNotConfigured) => {
            warn!("API send_text 失败: AI/辅助服务未配置。");
            Err(ApiError::AiNotConfigured)
        }
        Err(CoreError::NoTaskRunning) => {
            warn!("API send_text 失败: 没有正在运行的任务来发送文本。");
            Err(ApiError::ServiceUnavailable("没有活动任务可接收文本。请先启动一个任务。".to_string()))
        }
        Err(CoreError::TxQueueSendError(msg)) => {
            error!("API send_text 失败: 无法将项目发送到 TX 队列: {}", msg);
            Err(ApiError::InternalServerError(format!("无法将文本排队进行传输: {}", msg)))
        }
        Err(CoreError::AuxServiceNotConfigured(msg)) => {
            warn!("API send_text 失败: 辅助服务未配置: {}", msg);
            Err(ApiError::ServiceUnavailable(format!("辅助服务未配置: {}", msg)))
        }
        Err(e) => {
            error!("通过 API 调用排队文本进行传输时出错: {:?}", e);
            Err(ApiError::InternalServerError(format!("排队文本进行传输失败: {}", e)))
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

    let values_to_save = JsonValue::Object(payload.updates.into_iter().collect());

    match save_user_config_values(values_to_save) {
        Ok(_) => {
            info!("Configuration updated successfully via API.");
            Ok(StatusCode::OK)
        }
        Err(config_err) => {
            error!("Failed to save configuration via API: {}", config_err);
            match config_err {
                ElfConfigError::IoError(io_err) => Err(ApiError::InternalServerError(format!(
                    "Failed to write or access configuration file/directory: {}",
                    io_err
                ))),
                ElfConfigError::Config(rs_err) => {
                     if rs_err.to_string().contains("Invalid user TOML format") {
                          Err(ApiError::BadRequest(format!(
                               "Failed to save: Invalid existing config format. Please check/reset config file. Error: {}",
                               rs_err
                          )))
                     } else if rs_err.to_string().contains("Expected a JSON object") {
                           Err(ApiError::BadRequest(format!(
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
            }
        }
    }
}

/// Temporary handler for testing the translation service.
pub async fn test_translate_handler(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<TestTranslateRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let task_id_for_log = "TestTranslateHandler_Call"; // 用于日志记录上下文的占位符 TaskID
    info!(%task_id_for_log, "收到 /api/test/translate 请求。文本: '{}', 目标语言: '{}'", payload.text, payload.target_language);

    // 从 AppState 访问 MPSC 发送器
    let log_tx = app_state.log_entry_tx_for_handlers.clone();
    let status_tx = app_state.status_update_tx_for_handlers.clone();

    let aux_client_guard = app_state.aux_client.read().await;

    if let Some(client_to_use) = aux_client_guard.as_ref() {
        // client_to_use 是 &Arc<dyn AuxServiceClient + Send + Sync>
        debug!(%task_id_for_log, "尝试通过 AuxServiceClient 进行翻译...");

        match client_to_use.translate(
            &payload.text,
            &payload.target_language,
            payload.source_language.as_deref(),
        ).await {
            Ok(translated_text) => {
                info!(%task_id_for_log, "通过 /api/test/translate 进行的翻译调用成功。");

                // 为翻译服务发送 Ok 状态更新
                let translate_ok_status_update = WebSocketMessage::TranslateStatusUpdate(SystemServiceStatus::Ok);
                if status_tx.send(translate_ok_status_update).is_err() {
                    error!(%task_id_for_log, "成功测试调用后，通过 MPSC 通道发送 TranslateStatusUpdate(Ok) 失败。");
                }
                
                Ok(Json(json!({ "status": "success", "translated_text": translated_text })))
            }
            Err(ai_error) => { // ai_error 是 elfradio_types::AiError
                let determined_translate_status = match &ai_error {
                    ElfAiError::AuthenticationError(_) | ElfAiError::ApiError { status: 401, .. } | ElfAiError::ApiError { status: 403, .. } => {
                        SystemServiceStatus::Warning
                    }
                    ElfAiError::ApiError { status: 429, .. } => {
                        SystemServiceStatus::Warning
                    }
                    ElfAiError::RequestError(_) | ElfAiError::ApiError { status: 500..=599, .. } | ElfAiError::ClientError(_) => {
                        SystemServiceStatus::Error
                    }
                    ElfAiError::ProviderNotSpecified | ElfAiError::Config(_) => {
                        // 对于测试处理器，如果提供者未指定或配置错误，则将其视为警告，因为服务可能稍后会配置好
                        SystemServiceStatus::Warning
                    }
                    ElfAiError::ResponseParseError(_) => {
                        SystemServiceStatus::Error
                    }
                     // 添加对 InvalidInput 的处理
                    ElfAiError::InvalidInput(_) => {
                        SystemServiceStatus::Ok // 服务本身可能没问题，只是输入无效
                    }
                    _ => SystemServiceStatus::Error,
                };

                let log_message = format!(
                    "翻译服务运行时错误 (来自 /api/test/translate): 调用 translate 失败。确定的状态: {:?}. 详情: {:?}",
                    determined_translate_status,
                    ai_error
                );
                error!(%task_id_for_log, "{}", log_message);

                let translate_error_log_entry = LogEntry {
                    timestamp: Utc::now(),
                    direction: LogDirection::Internal,
                    content_type: LogContentType::Status,
                    content: log_message,
                };
                if log_tx.send(translate_error_log_entry).is_err() {
                    error!(%task_id_for_log, "从 /api/test/translate 通过 MPSC 通道发送翻译运行时错误日志条目失败。");
                }

                let translate_status_update_msg = WebSocketMessage::TranslateStatusUpdate(determined_translate_status.clone());
                if status_tx.send(translate_status_update_msg).is_err() {
                    error!(%task_id_for_log, "从 /api/test/translate 通过 MPSC 通道为运行时错误发送 TranslateStatusUpdate 失败。");
                }
                
                // 将 AiError 映射到 ApiError 以用于 HTTP 响应 (重用原始 test_translate_handler 中的映射)
                match ai_error {
                    ElfAiError::NotSupported(msg) => Err(ApiError::BadRequest(format!("翻译操作不支持: {}", msg))),
                    ElfAiError::AuthenticationError(msg) => Err(ApiError::Unauthorized(msg)),
                    ElfAiError::ApiError { status, message } => Err(ApiError::BadGateway(status, message)),
                    ElfAiError::RequestError(msg) => Err(ApiError::InternalServerError(format!("翻译请求失败 (RequestError): {}", msg))),
                    ElfAiError::ResponseParseError(msg) => Err(ApiError::BadGateway(502, format!("解析翻译上游响应失败: {}", msg))),
                    ElfAiError::Config(msg) => Err(ApiError::InternalServerError(format!("翻译客户端配置错误: {}", msg))),
                    ElfAiError::ClientError(msg) => Err(ApiError::InternalServerError(format!("翻译客户端内部错误: {}", msg))),
                    ElfAiError::ProviderNotSpecified => Err(ApiError::ServiceUnavailable("测试调用未指定翻译提供程序。".to_string())),
                    ElfAiError::InvalidInput(msg) => Err(ApiError::BadRequest(format!("翻译请求的输入无效: {}", msg))),
                    _ => Err(ApiError::InternalServerError(format!("翻译调用因意外 AI 错误失败: {}", ai_error))),
                }
            }
        }
    } else {
        warn!(%task_id_for_log, "通过 /api/test/translate 进行的翻译运行时调用失败: AuxClient 未在 AppState 中配置/可用。");
        let translate_status_update_msg = WebSocketMessage::TranslateStatusUpdate(SystemServiceStatus::Warning);
        if status_tx.send(translate_status_update_msg).is_err() {
            error!(%task_id_for_log, "从 /api/test/translate 通过 MPSC 通道发送 TranslateStatusUpdate (未配置) 失败。");
        }
        Err(ApiError::ServiceUnavailable("辅助服务 (用于翻译) 未初始化或未配置。请检查服务器日志和 AI 设置。".to_string()))
    }
}

/// Handler skeleton for testing the LLM service via /api/test/llm.
pub async fn test_llm_handler(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<TestLlmRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let task_id_for_log = "TestLLMHandler_Call"; // 用于日志记录上下文的占位符 TaskID
    info!(%task_id_for_log, "收到 /api/test/llm 请求。消息数量: {}", payload.messages.len());
    if let Some(ref params) = payload.params {
        debug!(%task_id_for_log, "LLM 测试 Payload 参数: {:?}", params);
    }

    // 从 AppState 访问 MPSC 发送器
    let log_tx = app_state.log_entry_tx_for_handlers.clone();
    let status_tx = app_state.status_update_tx_for_handlers.clone();

    let ai_client_guard = app_state.ai_client.read().await;

    if let Some(client_to_use) = ai_client_guard.as_ref() {
        // client_to_use 是 &Arc<dyn AiClient + Send + Sync>
        
        // 如果 payload.params 是 Some，则使用它，否则使用默认的 ChatParams
        // ChatParams 需要实现 Default trait
        let chat_params = payload.params.clone().unwrap_or_default(); 

        debug!(%task_id_for_log, "尝试通过 AiClient 进行 LLM 聊天补全...");

        match client_to_use.chat_completion(payload.messages, &chat_params).await {
            Ok(llm_response_text) => {
                info!(%task_id_for_log, "通过 /api/test/llm 进行的 LLM chat_completion 调用成功。");

                // 为 LLM 服务发送 Ok 状态更新
                let llm_ok_status_update = WebSocketMessage::LlmStatusUpdate(SystemServiceStatus::Ok);
                if status_tx.send(llm_ok_status_update).is_err() {
                    error!(%task_id_for_log, "成功测试调用后，通过 MPSC 通道发送 LlmStatusUpdate(Ok) 失败。");
                }
                
                Ok(Json(json!({ "status": "success", "response": llm_response_text })))
            }
            Err(ai_error) => { // ai_error 是 elfradio_types::AiError
                let determined_llm_status = match &ai_error {
                    ElfAiError::AuthenticationError(_) | ElfAiError::ApiError { status: 401, .. } | ElfAiError::ApiError { status: 403, .. } => {
                        SystemServiceStatus::Warning
                    }
                    ElfAiError::ApiError { status: 429, .. } => {
                        SystemServiceStatus::Warning
                    }
                    ElfAiError::RequestError(_) | ElfAiError::ApiError { status: 500..=599, .. } | ElfAiError::ClientError(_) => {
                        SystemServiceStatus::Error
                    }
                    ElfAiError::ProviderNotSpecified | ElfAiError::Config(_) => {
                        // 对于测试处理器，如果提供者未指定或配置错误，认为是更严重的问题
                        SystemServiceStatus::Error 
                    }
                    ElfAiError::ResponseParseError(_) => {
                        SystemServiceStatus::Error
                    }
                    // 添加对 InvalidInput 的处理，这通常是客户端错误，LLM 服务本身可能 OK
                    ElfAiError::InvalidInput(_) => {
                        SystemServiceStatus::Ok // 服务本身可能没问题，只是输入无效
                    }
                    _ => SystemServiceStatus::Error,
                };

                let log_message = format!(
                    "LLM 服务运行时错误 (来自 /api/test/llm): 调用 chat_completion 失败。确定的状态: {:?}. 详情: {:?}",
                    determined_llm_status,
                    ai_error
                );
                error!(%task_id_for_log, "{}", log_message);

                let llm_error_log_entry = LogEntry {
                    timestamp: Utc::now(),
                    direction: LogDirection::Internal,
                    content_type: LogContentType::Status,
                    content: log_message,
                };
                if log_tx.send(llm_error_log_entry).is_err() {
                    error!(%task_id_for_log, "从 /api/test/llm 通过 MPSC 通道发送 LLM 运行时错误日志条目失败。");
                }

                let llm_status_update_msg = WebSocketMessage::LlmStatusUpdate(determined_llm_status.clone()); // 克隆状态以用于日志和发送
                if status_tx.send(llm_status_update_msg).is_err() {
                    error!(%task_id_for_log, "从 /api/test/llm 通过 MPSC 通道为运行时错误发送 LlmStatusUpdate 失败。");
                }
                
                // 将 AiError 映射到 ApiError 以用于 HTTP 响应
                match ai_error {
                    ElfAiError::NotSupported(msg) => Err(ApiError::BadRequest(format!("LLM 操作不支持: {}", msg))),
                    ElfAiError::AuthenticationError(msg) => Err(ApiError::Unauthorized(msg)),
                    ElfAiError::ApiError { status, message } => Err(ApiError::BadGateway(status, message)),
                    ElfAiError::RequestError(msg) => Err(ApiError::InternalServerError(format!("LLM 请求失败 (RequestError): {}", msg))),
                    ElfAiError::ResponseParseError(msg) => Err(ApiError::BadGateway(502, format!("解析 LLM 上游响应失败: {}", msg))),
                    ElfAiError::Config(msg) => Err(ApiError::InternalServerError(format!("LLM 客户端配置错误: {}", msg))),
                    ElfAiError::ClientError(msg) => Err(ApiError::InternalServerError(format!("LLM 客户端内部错误: {}", msg))),
                    ElfAiError::ProviderNotSpecified => Err(ApiError::ServiceUnavailable("测试调用未指定 LLM 提供程序。".to_string())),
                    ElfAiError::InvalidInput(msg) => Err(ApiError::BadRequest(format!("LLM 请求的输入无效: {}", msg))),
                    _ => Err(ApiError::InternalServerError(format!("LLM 调用因意外 AI 错误失败: {}", ai_error))),
                }
            }
        }
    } else {
        warn!(%task_id_for_log, "通过 /api/test/llm 进行的 LLM 运行时调用失败: AiClient 未在 AppState 中配置/可用。");
        // 如果客户端未配置，发送 Warning 状态
        let llm_status_update_msg = WebSocketMessage::LlmStatusUpdate(SystemServiceStatus::Warning);
        if status_tx.send(llm_status_update_msg).is_err() {
            error!(%task_id_for_log, "从 /api/test/llm 通过 MPSC 通道发送 LlmStatusUpdate (未配置) 失败。");
        }
        Err(ApiError::ServiceUnavailable("LLM 客户端未初始化或未配置。请检查服务器日志和 AI 设置。".to_string()))
    }
}

// 在函数体内添加这个辅助函数，或者在模块级别添加
fn message_type_for_debug(msg: &WebSocketMessage) -> String {
    match msg {
        WebSocketMessage::Log(_) => "日志".to_string(),
        WebSocketMessage::RadioStatusUpdate(_) => "无线电状态更新".to_string(),
        WebSocketMessage::SdrStatusUpdate(_) => "SDR状态更新".to_string(),
        WebSocketMessage::LlmStatusUpdate(_) => "LLM状态更新".to_string(),
        WebSocketMessage::SttStatusUpdate(_) => "STT状态更新".to_string(),
        WebSocketMessage::TtsStatusUpdate(_) => "TTS状态更新".to_string(),
        WebSocketMessage::TranslateStatusUpdate(_) => "翻译状态更新".to_string(),
        WebSocketMessage::NetworkConnectivityUpdate(_) => "网络连接状态更新".to_string(),
        WebSocketMessage::UserUuidUpdate(_) => "用户UUID更新".to_string(),
        // 添加其他现有的变体（如果有的话）
    }
}
