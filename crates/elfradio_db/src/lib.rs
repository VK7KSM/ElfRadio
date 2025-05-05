use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use sqlx::pool::{PoolOptions};
use sqlx::sqlite::{SqliteConnectOptions};
use std::str::FromStr;
use tracing::info;
use thiserror::Error;
use elfradio_types::LogEntry;
use uuid::Uuid;
use tracing::instrument;
use elfradio_types::TaskInfo;
use chrono::Utc;
use tracing::warn;
use std::path::Path;
use std::fs;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Database migration failed: {0}")]
    MigrationFailed(String),
    
    #[error("Database query failed: {0}")]
    QueryFailed(String),
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Database operation failed: {0}")]
    OperationFailed(String),
    
    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),
    
    #[error("Invalid data format: {0}")]
    InvalidData(String),
}

impl From<sqlx::Error> for DbError {
    fn from(error: sqlx::Error) -> Self {
        DbError::QueryFailed(error.to_string())
    }
}

impl From<std::io::Error> for DbError {
    fn from(error: std::io::Error) -> Self {
        DbError::IoError(error.to_string())
    }
}

#[allow(dead_code)]
const DB_FILE_NAME: &str = "elfradio_data.db";

/// Initializes the SQLite database connection pool.
/// Creates the database file and runs migrations if necessary.
#[instrument]
pub async fn init_db(db_url: &str) -> Result<SqlitePool, DbError> {
    // 添加处理目录的逻辑
    if db_url.starts_with("sqlite:") {
        let file_path = db_url.trim_start_matches("sqlite:");
        
        // 提取文件的父目录
        if let Some(parent) = Path::new(file_path).parent() {
            if !parent.exists() {
                info!("Creating directory structure for database: {:?}", parent);
                fs::create_dir_all(parent)
                    .map_err(|e| DbError::IoError(format!("Failed to create database directory: {}", e)))?;
            }
        }
    }

    if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
        info!("Database not found, creating new one at {}", db_url);
        Sqlite::create_database(db_url)
            .await
            .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
    } else {
        info!("Using existing database at {}", db_url);
    }

    let connection_options = SqliteConnectOptions::from_str(db_url)
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = PoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;

    info!("Database pool created. Running migrations...");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY NOT NULL, -- UUID stored as TEXT
            name TEXT NOT NULL,
            mode TEXT NOT NULL,
            start_time TEXT NOT NULL, -- ISO8601 DateTime stored as TEXT
            end_time TEXT,
            task_dir TEXT NOT NULL UNIQUE,
            is_simulation BOOLEAN NOT NULL,
            metadata_json TEXT -- Store other metadata as JSON blob?
        );
        "#
    )
    .execute(&pool)
    .await
    .map_err(|e| DbError::MigrationFailed(e.to_string()))?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS log_entries (
            entry_id TEXT PRIMARY KEY NOT NULL, -- UUID for the log entry itself
            task_id TEXT NOT NULL,             -- Foreign key to tasks table
            timestamp TEXT NOT NULL,           -- ISO8601 DateTime string
            direction TEXT NOT NULL,           -- "Incoming", "Outgoing", "Internal"
            content_type TEXT NOT NULL,        -- "Text", "Audio", "Status", etc.
            content TEXT NOT NULL,             -- Text content or file path
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&pool)
    .await
    .map_err(|e| DbError::MigrationFailed(e.to_string()))?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_log_entries_task_id ON log_entries (task_id);
        "#,
    )
    .execute(&pool)
    .await
    .map_err(|e| DbError::MigrationFailed(e.to_string()))?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_log_entries_timestamp ON log_entries (timestamp);
        "#,
    )
    .execute(&pool)
    .await
    .map_err(|e| DbError::MigrationFailed(e.to_string()))?;

    info!("Database initialized and migrations run successfully.");
    Ok(pool)
}

/// Inserts a single log entry associated with a task into the database.
#[instrument(skip(pool, entry), fields(task_id = %task_id, entry_id))]
pub async fn insert_log_entry(
    pool: &SqlitePool,
    task_id: Uuid,
    entry: &LogEntry,
) -> Result<(), DbError> {
    let entry_id = Uuid::new_v4();
    tracing::Span::current().record("entry_id", &tracing::field::display(entry_id));

    let direction_str = format!("{:?}", entry.direction);
    let content_type_str = format!("{:?}", entry.content_type);
    let timestamp_str = entry.timestamp.to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO log_entries (entry_id, task_id, timestamp, direction, content_type, content)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(entry_id.to_string())
    .bind(task_id.to_string())
    .bind(timestamp_str)
    .bind(direction_str)
    .bind(content_type_str)
    .bind(&entry.content)
    .execute(pool)
    .await?;

    tracing::debug!("Successfully inserted log entry");
    Ok(())
}

/// 将新任务记录插入数据库。
#[instrument(skip(pool, task_info), fields(task_id = %task_info.id))]
pub async fn insert_task(pool: &SqlitePool, task_info: &TaskInfo) -> Result<(), DbError> {
    let task_id_str = task_info.id.to_string();
    let mode_str = format!("{:?}", task_info.mode);
    let start_time_str = Utc::now().to_rfc3339();
    let task_dir_str = task_info.task_dir.to_string_lossy().into_owned();

    sqlx::query(
        r#"
        INSERT INTO tasks (id, name, mode, start_time, task_dir, is_simulation)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(task_id_str.clone())
    .bind(&task_info.name)
    .bind(mode_str)
    .bind(start_time_str)
    .bind(task_dir_str)
    .bind(task_info.is_simulation)
    .execute(pool)
    .await?;
    
    info!("Successfully inserted task record for task_id: {}", task_id_str);
    Ok(())
}

/// 更新指定任务 ID 的结束时间。
#[instrument(skip(pool), fields(task_id = %task_id))]
pub async fn update_task_end_time(pool: &SqlitePool, task_id: Uuid) -> Result<(), DbError> {
    let task_id_str = task_id.to_string();
    let end_time_str = Utc::now().to_rfc3339();

    let result = sqlx::query(
        "UPDATE tasks SET end_time = $1 WHERE id = $2"
    )
    .bind(end_time_str)
    .bind(task_id_str.clone())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        warn!("尝试更新不存在的任务ID的结束时间: {}", task_id);
    } else {
        info!("Successfully updated end_time for task_id: {}", task_id_str);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use elfradio_types::{LogEntry, LogDirection, LogContentType};
    use uuid::Uuid;
    use chrono::Utc;
    use sqlx::{sqlite::SqlitePoolOptions, Row};
    use std::path::PathBuf;

    /// 辅助函数：创建测试所需的数据库表
    async fn create_test_tables(pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
        // 创建 tasks 表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY NOT NULL, 
                name TEXT NOT NULL, 
                mode TEXT NOT NULL,
                start_time TEXT NOT NULL, 
                end_time TEXT, 
                task_dir TEXT NOT NULL UNIQUE,
                is_simulation BOOLEAN NOT NULL, 
                metadata_json TEXT
            );
            "#
        ).execute(pool).await?;

        // 创建 log_entries 表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS log_entries (
                entry_id TEXT PRIMARY KEY NOT NULL, 
                task_id TEXT NOT NULL, 
                timestamp TEXT NOT NULL,
                direction TEXT NOT NULL, 
                content_type TEXT NOT NULL, 
                content TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );
            "#
        ).execute(pool).await?;

        Ok(())
    }

    /// 测试：验证能否在内存数据库中创建所需的表
    #[tokio::test]
    async fn test_init_db_in_memory() {
        // 使用内存数据库进行表创建测试
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to connect to in-memory DB");

        // 运行表创建逻辑
        create_test_tables(&pool)
            .await
            .expect("Failed to create test tables");

        // 通过查询验证表存在（应该有0行）
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(task_count, 0);
        
        let log_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM log_entries")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(log_count, 0);

        pool.close().await;
    }

    /// 测试：验证能否成功插入日志条目
    #[tokio::test]
    async fn test_insert_log_entry_success() {
        // 使用内存数据库
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to connect to in-memory DB");

        // 创建必要的表
        create_test_tables(&pool)
            .await
            .expect("Failed to create test tables");

        // 创建测试任务 ID 和任务信息
        let task_id = Uuid::new_v4();
        let task_dir = PathBuf::from(format!("/tmp/test/{}", task_id));
        
        // 首先插入一个任务记录(因为有外键约束)
        sqlx::query(
            r#"
            INSERT INTO tasks 
            (id, name, mode, start_time, end_time, task_dir, is_simulation, metadata_json)
            VALUES (?, ?, ?, ?, NULL, ?, ?, NULL)
            "#
        )
        .bind(task_id.to_string())
        .bind("Test Task")
        .bind("regular")
        .bind(Utc::now().to_rfc3339())
        .bind(task_dir.to_str().unwrap())
        .bind(false)
        .execute(&pool)
        .await
        .expect("Failed to insert test task");

        // 创建测试日志条目
        let test_entry = LogEntry {
            timestamp: Utc::now(),
            direction: LogDirection::Incoming,
            content_type: LogContentType::Text,
            content: "Test log message".to_string(),
        };

        // 调用被测试的函数
        let result = insert_log_entry(&pool, task_id, &test_entry).await;
        
        // 验证插入成功
        assert!(result.is_ok());

        // 查询数据库以验证条目已插入
        let log_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM log_entries")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(log_count, 1);

        // 验证插入的数据是否正确
        let row = sqlx::query("SELECT task_id, direction, content_type, content FROM log_entries")
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch log entry");
        
        let db_task_id: String = row.get("task_id");
        let direction: String = row.get("direction");
        let content_type: String = row.get("content_type");
        let content: String = row.get("content");

        assert_eq!(db_task_id, task_id.to_string());
        assert_eq!(direction, "Incoming");
        assert_eq!(content_type, "Text");
        assert_eq!(content, "Test log message");

        pool.close().await;
    }
}
