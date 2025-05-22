// --- Module Declarations & Re-exports (Ensure NO Duplicates) ---
pub mod error;
pub use error::CoreError;
pub mod state; // Declared ONCE at the top
pub use state::AppState; // Re-exported ONCE at the top
pub mod audio_processor;
// pub use audio_processor::audio_input_processor; // Removed as per instructions
pub mod tx_processor;
pub use tx_processor::{tx_queue_processor, queue_text_for_transmission};
pub mod logging;
// pub mod audio_input_handler; // Remove or comment out this incorrect line
pub mod task_manager; // 添加新的 task_manager 模块声明
pub mod network_monitor; // <--- 新增网络监控模块声明

// 导出audio_processor中的函数，以便主应用程序可以使用
pub use audio_processor::audio_input_processor; // 修正：使用正确的函数名
pub use network_monitor::check_initial_network_connectivity; // <--- 新增导出

// --- Necessary Imports (Cleaned) ---
use std::sync::Arc;
use tokio::sync::watch; // Add watch import
use tracing::{error, info, warn, instrument};

// --- Crate-wide Result Type ---
pub type Result<T> = std::result::Result<T, CoreError>;

// --- Core Functions (Minimal) ---
pub fn add(left: u64, right: u64) -> u64 { // Example function
    left + right
}

// --- Example/Helper Functions (Remove or keep if needed) ---
// struct CoreStateNeedsAi { ai_client: Arc<dyn AiClient>,}
// async fn example_tts_call(...) { ... }

// --- Main Logic Runner ---
#[instrument(skip(app_state, shutdown_rx), fields(app_name = %app_state.config.app_name))]
pub async fn run_core_logic(
    app_state: Arc<AppState>,
    mut shutdown_rx: watch::Receiver<bool>, // Add shutdown_rx parameter
) -> Result<()> {
    info!("Core logic started.");

    // --- Retrieve TX Receiver ---
    let tx_receiver = app_state.take_tx_receiver().await.ok_or_else(|| {
        CoreError::ConfigError("TX queue receiver missing or already taken from AppState".to_string())
    })?;

    // --- Spawn Transmit Queue Processor ---
    let tx_app_state = app_state.clone();
    // Create a *new* receiver specifically for the TX processor task
    let shutdown_rx_tx = shutdown_rx.clone(); // Clone the receiver for the spawned task
    let queue_handle = tokio::spawn(async move {
        // Pass the shutdown receiver to the processor
        tx_processor::tx_queue_processor(tx_receiver, tx_app_state, shutdown_rx_tx).await
        // Note: tx_queue_processor itself now uses select! and handles shutdown
    });

    // --- Core Logic Main Loop (Example: Wait for shutdown or TX task completion) ---
    // This loop demonstrates how run_core_logic itself can be shutdown-aware.
    // Adjust based on what run_core_logic actually needs to do.
    tokio::select! {
        _ = shutdown_rx.changed() => {
            if *shutdown_rx.borrow() {
                info!("Shutdown signal received in core logic main loop. Signaling TX queue to stop (if not already signaled).");
                // Optionally, signal TX queue processor handle to abort if needed,
                // but the internal select should handle graceful shutdown.
                // queue_handle.abort();
            }
        }
        res = queue_handle => {
            match res {
                 Ok(_) => info!("TX Queue processor completed normally."),
                 Err(e) => {
                     if e.is_panic() { error!("TX Queue processor task panicked!"); }
                     else if e.is_cancelled() { warn!("TX Queue processor task was cancelled."); }
                     else { error!("TX Queue processor task failed to join: {:?}", e); }
                 }
            }
        }
    }

    info!("Core logic finished.");
    Ok(())
}

// --- Test Module ---
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

// --- NO DUPLICATE mod state or pub use state::AppState HERE ---
// The duplicate lines that caused E0428 and E0252 should be gone.

// --- Add the correct module declaration ---
// Use 'mod' as requested, requires 'pub use' for public items
// mod audio_processor; // 删除这行 - 已在第6行声明过

// Declare other modules...
// pub mod dsp; // Example
// pub mod hardware_abstraction; // Example

// Re-export other important types if desired
// pub use state::AppState;

// 修改2：添加 task_manager 函数的导出
pub use task_manager::{start_task, stop_task};
