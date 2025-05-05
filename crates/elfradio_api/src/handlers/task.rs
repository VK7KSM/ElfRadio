use crate::error::ApiError;
use axum::{
    extract::{Path, Extension},
    http::{header, StatusCode},
    response::IntoResponse,
};
use elfradio_db::DbError;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::{
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
};
use tokio::task;
use tracing::{info, warn};
use uuid::Uuid;
use zip::{write::FileOptions, ZipWriter};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, FromRow)]
struct TaskFromDb {
    id: String,
    name: String,
    mode: String,
    start_time: String,
    end_time: Option<String>,
    task_dir: String,
    is_simulation: bool,
    metadata_json: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
struct LogEntryFromDb {
    entry_id: String,
    task_id: String,
    timestamp: String,
    direction: String,
    content_type: String,
    content: String,
}

pub async fn export_task_data_handler(
    Path(task_id): Path<Uuid>,
    Extension(app_state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    info!(task_id = %task_id, "Exporting task data");
    
    let db_pool = app_state.db_pool.clone();
    let task_id_str = task_id.to_string();
    
    let task_info = sqlx::query_as!(
        TaskFromDb,
        "SELECT id, name, mode, start_time, end_time, task_dir, is_simulation, metadata_json FROM tasks WHERE id = ?",
        task_id_str
    )
    .fetch_optional(&db_pool)
    .await
    .map_err(|e| ApiError::DatabaseError(e.to_string()))?
    .ok_or_else(|| ApiError::TaskNotFound(task_id))?;
    
    let log_entries = sqlx::query_as!(
        LogEntryFromDb,
        "SELECT timestamp, direction, content_type, content FROM log_entries WHERE task_id = ? ORDER BY timestamp ASC",
        task_id_str
    )
    .fetch_all(&db_pool)
    .await
    .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
    
    let mut log_content = String::new();
    for entry in &log_entries {
        match serde_json::to_string(entry) {
            Ok(json_line) => {
                log_content.push_str(&json_line);
                log_content.push('\n');
            }
            Err(e) => warn!(task_id = %task_id, "Failed to serialize log entry: {:?}", e),
        }
    }
    
    let task_dir = PathBuf::from(&task_info.task_dir);
    
    let zip_data = task::spawn_blocking(move || -> Result<Vec<u8>, io::Error> {
        let mut zip_buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
            let options = FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o644);
            
            zip.start_file("task_info.json", options)?;
            match serde_json::to_string_pretty(&task_info) {
                Ok(task_info_json) => {
                    zip.write_all(task_info_json.as_bytes())?;
                }
                Err(e) => {
                    return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to serialize task info: {}", e)));
                }
            }
            
            zip.start_file("events.jsonl", options)?;
            zip.write_all(log_content.as_bytes())?;
            
            if task_dir.exists() {
                let audio_dir = "audio";
                
                for entry in std::fs::read_dir(&task_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            if ext == "wav" {
                                let file_name = path.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy();
                                
                                let zip_path = format!("{}/{}", audio_dir, file_name);
                                zip.start_file(zip_path, options)?;
                                
                                let file_content = std::fs::read(&path)?;
                                zip.write_all(&file_content)?;
                            }
                        }
                    }
                }
            }
            
            zip.finish()?;
        }
        
        Ok(zip_buffer)
    })
    .await
    .map_err(|e| ApiError::InternalError(format!("Task join error: {}", e)))?
    .map_err(|e| ApiError::InternalError(format!("IO error during ZIP creation: {}", e)))?;
    
    info!(task_id = %task_id, "Task data exported successfully");
    
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"task_{}.zip\"", task_id),
            ),
        ],
        zip_data,
    ))
} 