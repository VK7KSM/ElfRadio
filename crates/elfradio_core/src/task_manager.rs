use std::sync::Arc;
use tokio::fs; // For async filesystem operations
use chrono::Utc; // Changed to Utc
use uuid::Uuid;
use tracing::{info, warn, error, instrument}; // 删除 debug 导入
use crate::state::AppState;
use crate::error::CoreError;
use elfradio_types::{TaskMode, TaskInfo, TaskStatus}; // 删除 AudioMessage 导入
// 新增导入
// use elfradio_hardware::{start_audio_input_stream, start_audio_output_stream, HardwareError};
// use tokio::sync::mpsc;
// use cpal::traits::{HostTrait, DeviceTrait};
// 修改1：添加std::time::Instant导入
// --- 添加: 导入数据库插入函数 ---
use elfradio_db::insert_task;
// --- 添加结束 ---
// --- 添加: 导入数据库更新函数 ---
use elfradio_db::update_task_end_time;
// --- 添加结束 ---

/// Attempts to start a new task, creating its directory and updating the application state.
/// Inserts the task information into the database.
///
/// Returns the `Uuid` of the newly created task if successful.
#[instrument(skip(app_state), fields(mode = ?mode))]
pub async fn start_task(app_state: Arc<AppState>, mode: TaskMode) -> Result<Uuid, CoreError> {
    info!("Attempting to start task.");

    // --- Check State ---
    let mut status_guard = app_state.task_status.lock().await;
    if *status_guard != TaskStatus::Idle {
        warn!("Cannot start task: Another task is already running or stopping. Current status: {:?}", *status_guard);
        return Err(CoreError::TaskAlreadyRunning);
    }
    
    // --- Create Task Info & Directory ---
    let task_id = Uuid::new_v4();
    // Use UTC time for consistency, format for directory name
    let timestamp = Utc::now().format("%Y%m%d_%H%M%SZ").to_string();
    // 修改2：使用{:?}替代{}来格式化mode
    let task_name = format!("{:?}_{}_{}", mode, timestamp, task_id.simple());
    
    // Corrected: Access config directly as it's Arc<Config>
    let task_dir = app_state.config.tasks_base_directory.join(&task_name);

    info!("Creating task directory: {:?}", task_dir);
    // Propagates io::Error converted to CoreError::IoError via #[from]
    fs::create_dir_all(&task_dir).await?; 

    // 修改3：确保使用正确的TaskMode变体
    let is_simulation = matches!(mode, TaskMode::SimulatedQsoPractice);

    let task_info = TaskInfo {
        id: task_id,
        name: task_name.clone(), // Clone name for logging/DB insert
        mode,
        // 修改4：使用std::time::Instant而不是tokio::time::Instant
        start_time: std::time::Instant::now(),
        task_dir: task_dir.clone(), // Clone path
        is_simulation, // 修改5：确保is_simulation字段正确初始化
    };
    
    // --- Insert task record into database ---
    // Access db_pool directly from app_state
    if let Err(e) = insert_task(&app_state.db_pool, &task_info).await {
        // Log the error, but continue starting the task for now.
        // Consider returning CoreError::DatabaseError(e.to_string()) if DB write failure is critical.
        error!(task_id = %task_info.id, "Failed to insert task into database: {:?}", e);
        // Optional: return Err(CoreError::DatabaseError(e.to_string())); 
    } else {
        info!(task_id = %task_info.id, "Task info inserted into database.");
    }
    // --- Database insertion end ---

    // --- TODO Placeholders ---
    // TODO: Start hardware audio streams based on config (app_state.config) and task_info.is_simulation flag
    // TODO: Spawn specific task processors if needed (e.g., audio_input_processor if not started globally, passing necessary state like task_info or app_state clone)

    // --- Update AppState ---
    // 删除与音频流控制相关的代码
    // *app_state.input_stream_control.lock().await = Some(input_control);
    // *app_state.output_stream_control.lock().await = Some(output_control);
    // *app_state.audio_output_sender.lock().await = Some(output_sender);
    
    // 更新任务状态
    *status_guard = TaskStatus::Running;
    // Drop the status guard explicitly before acquiring the next lock
    drop(status_guard);

    let mut active_task_guard = app_state.active_task.lock().await;
    *active_task_guard = Some(task_info.clone());
    // Drop the active_task guard explicitly
    drop(active_task_guard);

    info!(task_id = %task_id, task_name = %task_name, "Task started successfully.");
    Ok(task_id)
}

/// Stops the currently running task, updates its end time in the database, and resets the application state.
///
/// Returns `Ok(())` if the task was stopped successfully or if no task was running.
/// Returns an error if stopping fails unexpectedly.
#[instrument(skip(app_state))]
pub async fn stop_task(app_state: Arc<AppState>) -> Result<(), CoreError> {
    info!("Attempting to stop the current task.");

    // --- Check State ---
    let mut status_guard = app_state.task_status.lock().await;
    if *status_guard != TaskStatus::Running {
        warn!("Cannot stop task: No task is currently running. Status: {:?}", *status_guard);
        return Err(CoreError::NoTaskRunning);
    }
    *status_guard = TaskStatus::Stopping; // Mark as stopping
    drop(status_guard); // Release lock early

    // --- Retrieve Task Info ---
    let mut active_task_guard = app_state.active_task.lock().await;
    let task_info_option = active_task_guard.take(); // Take ownership
    drop(active_task_guard); // Release lock

    let task_id_to_stop = if let Some(ref info) = task_info_option {
        info!(task_id = %info.id, task_name = %info.name, "Stopping task.");
        Some(info.id) // Keep the ID for DB update
    } else {
        // This case should ideally not happen if status was Running, but handle defensively
        error!("Inconsistent state: TaskStatus was Running but ActiveTask was None.");
        // Reset status back to Idle
        let mut status_guard = app_state.task_status.lock().await;
        *status_guard = TaskStatus::Idle;
        return Err(CoreError::InvalidState("Active task info missing while stopping.".to_string()));
    };

    // --- TODO Placeholders ---
    // TODO: Gracefully stop hardware audio streams...
    // TODO: Signal and await termination of task-specific processors...
    // TODO: Perform any final data flushing or cleanup related to the task...
    // Example: Simulating some cleanup work
    // tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    info!(task_id = ?task_id_to_stop, "Task-specific cleanup completed (placeholder).");

    // --- Update task end time in database ---
    if let Some(id) = task_id_to_stop {
        if let Err(e) = update_task_end_time(&app_state.db_pool, id).await {
            error!(task_id = %id, "Failed to update task end time in database: {:?}", e);
            // Log error but continue stopping process
        } else {
            info!(task_id = %id, "Task end time updated in database.");
        }
    }
    // --- Database update end ---

    // --- Update AppState (Final Step) ---
    info!(task_id = ?task_id_to_stop, "Updating application state to Idle.");
    let mut status_guard = app_state.task_status.lock().await;
    *status_guard = TaskStatus::Idle; // Set final state
    // active_task was already cleared by .take() earlier

    info!(task_id = ?task_id_to_stop, "Task stopped successfully.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*; // Imports items from the parent module (task_manager)
    use crate::state::AppState;
    use crate::error::CoreError;
    // Ensure all necessary types from elfradio_types are imported
    use elfradio_types::{
        TaskMode, TaskStatus, Config, TaskInfo, AudioOutputSender, TxItem, // Add AudioMessage if needed by tests
    };
    use std::sync::Arc;
    // Ensure RwLock, OnceCell, broadcast are imported if used, mpsc, watch, Mutex definitely needed
    use tokio::sync::{mpsc, watch, Mutex, RwLock, OnceCell};
    
    // Ensure all necessary types from elfradio_ai are imported
    use elfradio_ai::{AiClient, ChatMessage, ChatParams, TtsParams, SttParams};
    use elfradio_types::AiError;  // 直接从 elfradio_types 导入 AiError
    use assert_matches::assert_matches;
    use uuid::Uuid;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use async_trait::async_trait;
    use std::collections::HashMap;
     // Ensure std::time::Instant is used

    #[derive(Debug, Clone)]
    struct DummyAiClient;

    #[async_trait]
    impl AiClient for DummyAiClient {
        async fn chat_completion(&self, _messages: Vec<ChatMessage>, _params: &ChatParams) -> Result<String, AiError> {
            Ok("dummy reply".to_string())
        }

        async fn text_to_speech(&self, _text: &str, _params: &TtsParams) -> Result<Vec<u8>, AiError> {
            Ok(vec![0u8; 10])
        }

        async fn speech_to_text(&self, _audio_data: &[u8], _params: &SttParams) -> Result<String, AiError> {
            Ok("dummy stt".to_string())
        }

        async fn list_models(&self) -> Result<Vec<String>, AiError> {
            Ok(vec!["dummy-model".to_string()])
        }
    }

    async fn create_test_app_state(temp_task_dir: PathBuf) -> Arc<AppState> {
        let (tx_sender, tx_receiver) = mpsc::unbounded_channel::<TxItem>();
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        let db_pool = elfradio_db::init_db("sqlite::memory:")
            .await
            .expect("Failed to initialize in-memory DB for test");

        let mut config = Config::default();
        config.tasks_base_directory = temp_task_dir;

        Arc::new(AppState {
            task_status: Mutex::new(TaskStatus::Idle),
            active_task: Mutex::new(None),
            tx_queue: tx_sender,
            config: Arc::new(config),
            db_pool: db_pool,
            ai_client: Arc::new(RwLock::new(Some(
                Arc::new(DummyAiClient {}) as Arc<dyn AiClient + Send + Sync>,
            ))),
            is_transmitting: Arc::new(Mutex::new(false)),
            audio_output_sender: Arc::new(Mutex::new(None::<AudioOutputSender>)),
            shutdown_tx,
            clients: Arc::new(Mutex::new(HashMap::new())),
            log_broadcast_task_handle: Arc::new(OnceCell::new()),
            tx_queue_rx: Mutex::new(Some(tx_receiver)),
        })
    }

    #[tokio::test]
    async fn test_start_task_success() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let app_state = create_test_app_state(temp_dir.path().to_path_buf()).await;
        let result = start_task(app_state.clone(), TaskMode::GeneralCommunication).await;
        let task_id = result.expect("Task start should succeed");
        let status = app_state.task_status.lock().await.clone();
        assert_eq!(status, TaskStatus::Running);
        let active_task = app_state.active_task.lock().await.clone();
        assert!(active_task.is_some(), "Active task should be set");
        let task_info = active_task.unwrap();
        assert_eq!(task_info.mode, TaskMode::GeneralCommunication);
        assert_eq!(task_info.id, task_id);
        assert!(task_info.task_dir.exists(), "Task directory should exist");
        let task_dir_str = task_info.task_dir.to_string_lossy();
        let temp_dir_str = temp_dir.path().to_string_lossy();
        assert!(task_dir_str.starts_with(&*temp_dir_str), // Check if task_dir starts with temp_dir path
            "Task directory '{}' should start with temp directory path '{}'", // Updated message slightly
            task_dir_str, temp_dir_str);
    }

    #[tokio::test]
    async fn test_start_task_already_running() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let app_state = create_test_app_state(temp_dir.path().to_path_buf()).await;
        *app_state.task_status.lock().await = TaskStatus::Running;
        let result = start_task(app_state.clone(), TaskMode::GeneralCommunication).await;
        assert_matches!(result, Err(CoreError::TaskAlreadyRunning));
    }

    #[tokio::test]
    async fn test_stop_task_success() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let app_state = create_test_app_state(temp_dir.path().to_path_buf()).await;
        let task_id = Uuid::new_v4();
        let task_dir = temp_dir.path().join(task_id.to_string());
        std::fs::create_dir_all(&task_dir).expect("Failed to create task directory");
        
        let dummy_task = TaskInfo {
            id: task_id,
            name: "Test Task".to_string(),
            mode: TaskMode::GeneralCommunication,
            start_time: std::time::Instant::now(),
            task_dir,
            is_simulation: false,
        };
        
        *app_state.task_status.lock().await = TaskStatus::Running;
        *app_state.active_task.lock().await = Some(dummy_task);
        
        let result = stop_task(app_state.clone()).await;
        assert!(result.is_ok(), "Task stop should succeed");
        assert_eq!(*app_state.task_status.lock().await, TaskStatus::Idle);
        assert!(app_state.active_task.lock().await.is_none(), "Active task should be None");
    }

    #[tokio::test]
    async fn test_stop_task_when_idle() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let app_state = create_test_app_state(temp_dir.path().to_path_buf()).await;
        assert_eq!(*app_state.task_status.lock().await, TaskStatus::Idle); 
        let result = stop_task(app_state.clone()).await;
        
        assert_matches!(result, Err(CoreError::NoTaskRunning), "Stopping when idle should return NoTaskRunning error");
        
        assert_eq!(*app_state.task_status.lock().await, TaskStatus::Idle);
    }
}
