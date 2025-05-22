use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock, OnceCell};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use elfradio_types::{
    Config,
    ClientMap, AudioOutputSender, TxItem, TaskInfo, TaskStatus,
    AuxServiceClient,
    LogEntry, WebSocketMessage,
};
use elfradio_ai::AiClient;
use sqlx::SqlitePool;

/// Shared application state accessible across tasks and handlers.
pub struct AppState {
    pub config: Arc<Config>,
    pub tx_queue: mpsc::UnboundedSender<TxItem>,
    pub tx_queue_rx: Mutex<Option<mpsc::UnboundedReceiver<TxItem>>>,
    pub clients: ClientMap,
    pub log_broadcast_task_handle: Arc<OnceCell<JoinHandle<()>>>,
    pub audio_output_sender: Arc<Mutex<Option<AudioOutputSender>>>,
    pub is_transmitting: Arc<Mutex<bool>>,
    pub ai_client: Arc<RwLock<Option<Arc<dyn AiClient + Send + Sync>>>>,
    pub aux_client: Arc<RwLock<Option<Arc<dyn AuxServiceClient + Send + Sync>>>>,
    pub active_task: Mutex<Option<TaskInfo>>,
    pub task_status: Mutex<TaskStatus>,
    pub shutdown_tx: watch::Sender<bool>,
    pub db_pool: SqlitePool,
    pub log_entry_tx_for_handlers: mpsc::UnboundedSender<LogEntry>,
    pub status_update_tx_for_handlers: mpsc::UnboundedSender<WebSocketMessage>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Arc<Config>,
        tx_queue_sender: mpsc::UnboundedSender<TxItem>,
        tx_queue_receiver: mpsc::UnboundedReceiver<TxItem>,
        clients: ClientMap,
        audio_output_sender: Arc<Mutex<Option<AudioOutputSender>>>,
        is_transmitting: Arc<Mutex<bool>>,
        shutdown_tx: watch::Sender<bool>,
        db_pool: SqlitePool,
        log_entry_tx_clone_for_handlers: mpsc::UnboundedSender<LogEntry>,
        status_update_tx_clone_for_handlers: mpsc::UnboundedSender<WebSocketMessage>
    ) -> Self {
        Self {
            config,
            tx_queue: tx_queue_sender,
            tx_queue_rx: Mutex::new(Some(tx_queue_receiver)),
            clients,
            log_broadcast_task_handle: Arc::new(OnceCell::new()),
            audio_output_sender,
            is_transmitting,
            ai_client: Arc::new(RwLock::new(None)),
            aux_client: Arc::new(RwLock::new(None)),
            active_task: Mutex::new(None),
            task_status: Mutex::new(TaskStatus::Idle),
            shutdown_tx,
            db_pool,
            log_entry_tx_for_handlers: log_entry_tx_clone_for_handlers,
            status_update_tx_for_handlers: status_update_tx_clone_for_handlers,
        }
    }

    pub async fn take_tx_receiver(&self) -> Option<mpsc::UnboundedReceiver<TxItem>> {
        let mut lock = self.tx_queue_rx.lock().await;
        lock.take()
    }

    pub fn get_tx_sender_placeholder(&self) -> Option<Arc<Mutex<Option<mpsc::UnboundedSender<TxItem>>>>> {
        None
    }

    pub async fn set_active_task(&self, task_info: Option<TaskInfo>) {
        let mut active_task_guard = self.active_task.lock().await;
        *active_task_guard = task_info;
    }

    pub async fn get_active_task_info(&self) -> Option<TaskInfo> {
        self.active_task.lock().await.clone()
    }
} 