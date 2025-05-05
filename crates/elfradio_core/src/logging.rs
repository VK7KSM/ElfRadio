use super::error::CoreError;
use elfradio_types::LogEntry;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;
use serde_json;
use tracing::error;

// Define a specific Result type alias for this module
type LoggingOutcome<T> = std::result::Result<T, CoreError>;

/// Writes a LogEntry struct to a JSON Lines file (e.g., events.jsonl).
///
/// Appends the serialized LogEntry as a new line to the specified file.
/// Creates the file if it doesn't exist.
pub fn write_log_entry(task_dir: &Path, entry: &LogEntry) -> LoggingOutcome<()> {
    // TODO: Determine the correct log file name (e.g., "events.jsonl")
    let log_file_path = task_dir.join("events.jsonl");
    
    // --- 添加: 确保任务目录存在 ---
    if !task_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(task_dir) {
            error!("Failed to create task directory {:?}: {}", task_dir, e);
            return Err(CoreError::IoError(e));
        }
    }
    // --- 添加结束 ---

    // Serialize the LogEntry to a JSON string
    let json_string = match serde_json::to_string(entry) {
        Ok(s) => s,
        Err(e) => {
            // Log the serialization error, but maybe don't stop core logic?
            error!("Failed to serialize LogEntry for writing: {}", e);
            // Return a specific error or handle differently
            return Err(CoreError::SerializationError(e));
        }
    };

    // Open the file in append mode, create if it doesn't exist
    let file = match OpenOptions::new()
        .append(true)
        .create(true)
        .open(&log_file_path)
    {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to open or create log file {:?}: {}", log_file_path, e);
            return Err(CoreError::IoError(e));
        }
    };

    // Use a BufWriter for potentially better performance
    let mut writer = BufWriter::new(file);

    // Write the JSON string as a line
    if let Err(e) = writeln!(writer, "{}", json_string) {
        error!("Failed to write log entry to {:?}: {}", log_file_path, e);
        return Err(CoreError::IoError(e));
    }

    // BufWriter is flushed when it goes out of scope

    Ok(())
}

// Optional: Add functions for initializing logging, rotating logs, etc.

#[cfg(test)]
mod tests {
    use super::*; // 导入 write_log_entry
    use elfradio_types::{LogContentType, LogDirection, LogEntry};
    use tempfile::tempdir;
    use chrono::Utc;
    use std::fs;
    use serde_json;

    #[tokio::test]
    async fn test_write_log_entry_creates_file_and_writes_jsonl() -> Result<(), Box<dyn std::error::Error>> {
        // 创建临时目录
        let temp_dir = tempdir()?;
        let task_dir = temp_dir.path().to_path_buf();

        // 创建示例日志条目
        let sample_entry = LogEntry {
            direction: LogDirection::Outgoing,
            content_type: LogContentType::Text,
            content: "Test log message".to_string(),
            timestamp: Utc::now(),
        };

        // 调用待测试函数
        write_log_entry(&task_dir, &sample_entry)
            .expect("写入日志条目应该成功");

        // 验证日志文件已创建
        let log_file = task_dir.join("events.jsonl");
        assert!(log_file.exists(), "日志文件应该存在");

        // 读取并解析日志文件内容
        let content = fs::read_to_string(&log_file)?;
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1, "应该只有一行日志");

        // 反序列化并比较
        let deserialized_entry: LogEntry = serde_json::from_str(lines[0])?;
        assert_eq!(deserialized_entry.direction, sample_entry.direction);
        assert_eq!(deserialized_entry.content_type, sample_entry.content_type);
        assert_eq!(deserialized_entry.content, sample_entry.content);
        // 注意：timestamp 可能有微小差异，所以不进行精确比较
        // 但可以检查是否在合理范围内（比如秒级）
        let time_diff = (deserialized_entry.timestamp - sample_entry.timestamp).num_seconds();
        assert!(time_diff.abs() < 1, "时间戳差异应该小于 1 秒");

        Ok(())
    }

    #[tokio::test]
    async fn test_write_log_entry_appends_to_existing_file() -> Result<(), Box<dyn std::error::Error>> {
        // 创建临时目录
        let temp_dir = tempdir()?;
        let task_dir = temp_dir.path().to_path_buf();

        // 创建两个不同的日志条目
        let entry1 = LogEntry {
            direction: LogDirection::Incoming,
            content_type: LogContentType::Text,
            content: "First log message".to_string(),
            timestamp: Utc::now(),
        };

        let entry2 = LogEntry {
            direction: LogDirection::Outgoing,
            content_type: LogContentType::Audio,
            content: "audio_file.wav".to_string(),
            timestamp: Utc::now(),
        };

        // 写入第一个条目
        write_log_entry(&task_dir, &entry1)
            .expect("写入第一个日志条目应该成功");

        // 写入第二个条目
        write_log_entry(&task_dir, &entry2)
            .expect("写入第二个日志条目应该成功");

        // 验证日志文件
        let log_file = task_dir.join("events.jsonl");
        assert!(log_file.exists(), "日志文件应该存在");

        // 读取并解析日志文件内容
        let content = fs::read_to_string(&log_file)?;
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2, "应该有两行日志");

        // 反序列化并比较第一个条目
        let deserialized_entry1: LogEntry = serde_json::from_str(lines[0])?;
        assert_eq!(deserialized_entry1.direction, entry1.direction);
        assert_eq!(deserialized_entry1.content_type, entry1.content_type);
        assert_eq!(deserialized_entry1.content, entry1.content);

        // 反序列化并比较第二个条目
        let deserialized_entry2: LogEntry = serde_json::from_str(lines[1])?;
        assert_eq!(deserialized_entry2.direction, entry2.direction);
        assert_eq!(deserialized_entry2.content_type, entry2.content_type);
        assert_eq!(deserialized_entry2.content, entry2.content);

        Ok(())
    }

    #[tokio::test]
    async fn test_write_log_entry_creates_directory_if_not_exists() -> Result<(), Box<dyn std::error::Error>> {
        // 创建临时目录
        let temp_dir = tempdir()?;
        // 创建一个不存在的子目录路径
        let task_dir = temp_dir.path().join("non_existent_subdir");
        assert!(!task_dir.exists(), "测试前子目录不应该存在");

        // 创建示例日志条目
        let entry = LogEntry {
            direction: LogDirection::Internal,
            content_type: LogContentType::Status,
            content: "System status".to_string(),
            timestamp: Utc::now(),
        };

        // 调用待测试函数
        write_log_entry(&task_dir, &entry)
            .expect("写入日志条目应该成功，并创建目录");

        // 验证目录和日志文件都已创建
        assert!(task_dir.exists(), "任务目录应该已被创建");
        let log_file = task_dir.join("events.jsonl");
        assert!(log_file.exists(), "日志文件应该存在");

        // 读取日志内容验证
        let content = fs::read_to_string(&log_file)?;
        let deserialized_entry: LogEntry = serde_json::from_str(&content.lines().next().unwrap())?;
        assert_eq!(deserialized_entry.direction, entry.direction);

        Ok(())
    }
} 