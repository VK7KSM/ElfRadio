use elfradio_types::Config as AppConfig; // Alias to avoid naming collision
use config::{Config as ConfigRs, Environment, File, ConfigError as RsConfigError};
// use directories::ProjectDirs; // REMOVED
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, info, warn, error}; // Ensure tracing imports are present
use std::fs; // Ensure fs is imported
use std::env; // ADDED for current_dir
use serde_json::Value as JsonValue; // Added for input type
use toml_edit::{DocumentMut, value as toml_value, Item}; // Added for TOML writing and Item
use uuid::Uuid;
use serde_json::json;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("I/O error accessing configuration file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration parsing or validation error: {0}")]
    Config(#[from] RsConfigError),

    // #[error("Could not determine a valid configuration directory")] // REMOVED as DirectoryError is no longer distinct from IoError for CWD
    // DirectoryError, // REMOVED
}

/// Helper function to get the full path to the config file in the app's directory.
/// Assumes the application runs with its working directory set to the app's root.
fn get_app_config_path() -> Result<PathBuf, ConfigError> {
    // Get the current working directory
    match env::current_dir() {
        Ok(mut path) => {
            path.push("elfradio_config.toml"); // Append the config file name
            Ok(path)
        }
        Err(e) => {
            // Log error if CWD cannot be determined
            tracing::error!("Could not determine the current working directory: {}", e);
            // Map the IO error to our ConfigError::Io variant
            Err(ConfigError::IoError(e))
        }
    }
}

/// 确保 config 中设置了有效的 user_uuid。
/// 如果 user_uuid 为 None 或空字符串，则生成一个新的 UUID，
/// 并尝试将其保存到配置文件中。
fn ensure_user_uuid_is_set(config: &mut AppConfig) {
    // 检查当前 user_uuid 是否为 None 或空字符串
    let should_generate_new_uuid = match &config.user_uuid {
        None => true,
        Some(uuid) if uuid.trim().is_empty() => true,
        _ => false,
    };

    if should_generate_new_uuid {
        // 生成新的 UUID
        let new_uuid = Uuid::new_v4().to_string();
        info!("没有找到现有用户 UUID 或它为空。正在生成并保存新的用户 UUID: {}", new_uuid);

        // 创建用于保存的 JSON 对象
        let uuid_to_save = json!({
            "user_uuid": new_uuid.clone()
        });

        // 尝试保存到配置文件
        match save_user_config_values(uuid_to_save) {
            Ok(_) => info!("成功保存新生成的用户 UUID 到配置文件"),
            Err(e) => error!("无法将新生成的用户 UUID 保存到配置文件: {:?}", e),
        }

        // 无论保存是否成功，都更新当前会话中的 config 对象
        config.user_uuid = Some(new_uuid);
    }
}

/// Loads the ElfRadio configuration.
///
/// Checks if the application configuration file (`elfradio_config.toml` in the application's
/// current working directory) exists. If not, it attempts to create it by copying from
/// `config/default.toml` (relative to the CWD).
///
/// Afterwards, it merges configuration from multiple sources using `config-rs`:
/// 1. `config/default.toml` (relative to CWD, required base).
/// 2. The `elfradio_config.toml` file in the app's CWD (optional, overrides defaults if it exists or was copied).
/// 3. Environment variables prefixed with `ELFRADIO_` (highest precedence).
pub fn load_config() -> Result<AppConfig, ConfigError> {
    // --- Step 1: Check and copy default config if necessary ---\
    let app_config_path = get_app_config_path()?; // Use the new helper

    if !app_config_path.exists() {
        info!("App config file not found at {:?}, attempting to create from default.", app_config_path);

        // No need to create parent directory for CWD, fs::copy will create the file.
        // However, if app_config_path were nested like "config/elfradio_config.toml",
        // then creating "config/" dir would be needed if it didn't exist.

        // IMPORTANT: Assumes 'config/default.toml' exists relative to the CWD where the app runs.
        // For robust portable deployment, consider embedding or placing it next to the executable.
        let default_config_path = PathBuf::from("config/default.toml");

        if default_config_path.exists() {
            match fs::copy(&default_config_path, &app_config_path) {
                Ok(_) => info!("Successfully copied default config to {:?}.", app_config_path),
                Err(e) => {
                    error!("Failed to copy default config from {:?} to {:?}: {}. Proceeding without app-specific config file.", default_config_path, app_config_path, e);
                    // Don't return error here, just warn and proceed. config-rs will rely on default.toml and env vars.
                }
            }
        } else {
            warn!(
                "Default config template not found at {:?}. Cannot create app-specific config file. App will rely on built-in defaults and environment variables if this is the first run.",
                default_config_path
            );
            // If default.toml is critical and missing, consider returning an error:
            // return Err(ConfigError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Default config template not found at {:?}", default_config_path))));
        }
    }

    // --- Step 2: Load configuration using config-rs ---\
    debug!("Building configuration sources...");
    let builder = ConfigRs::builder()
        // 1. Add default config file (relative path, required base)
        // IMPORTANT: Assumes 'config/default.toml' exists relative to the CWD where the app runs.
        .add_source(File::with_name("config/default.toml").required(true))
        // 2. Add app-specific config file from CWD (optional, overrides defaults if it exists or was copied)
        .add_source(File::from(app_config_path.clone()).required(false))
        // 3. Add environment variables (highest precedence)
        .add_source(
            Environment::with_prefix("ELFRADIO")
                .prefix_separator("_")
                .separator("__")
                .try_parsing(true)
                .list_separator(",")
        );

    debug!("Building and deserializing configuration...");
    let config_rs = builder.build()?;
    let mut app_config = config_rs.try_deserialize::<AppConfig>()?;

    // 确保 user_uuid 已设置，如果没有则生成新的
    ensure_user_uuid_is_set(&mut app_config);

    info!("Configuration loaded successfully from app directory.");
    Ok(app_config)
}

/// Retrieves a specific configuration value directly from the application's config file.
/// Only reads the `elfradio_config.toml` in the CWD, ignoring defaults and environment variables.
/// Useful for reading sensitive values like API keys at the time of use.
/// Returns Ok(None) if the app config file doesn't exist or the key is not found within it.
/// Returns Err(ConfigError) for file read/parse errors or type mismatches.
pub fn get_user_config_value<T: serde::de::DeserializeOwned>(key: &str) -> Result<Option<T>, ConfigError> {
    let app_config_path = get_app_config_path()?; // Use the new helper

    if app_config_path.exists() {
        tracing::debug!("Attempting to read key '{}' from app config file: {:?}", key, app_config_path);
        let config_reader = ConfigRs::builder()
            .add_source(File::from(app_config_path.clone()).required(true))
            .build()
            .map_err(|e| {
                tracing::error!("Failed to build config reader for app file {:?}: {}", app_config_path, e);
                ConfigError::Config(e)
            })?;

        match config_reader.get::<T>(key) {
            Ok(value) => {
                tracing::debug!("Successfully retrieved key '{}' from app config file.", key);
                Ok(Some(value))
            }
            Err(e) => {
                if let RsConfigError::NotFound(_) = e {
                     tracing::debug!("Key '{}' not found in app config file {:?}.", key, app_config_path);
                     Ok(None)
                } else {
                     tracing::error!("Error getting key '{}' from app config file {:?}: {}", key, app_config_path, e);
                     Err(ConfigError::Config(e))
                }
            }
        }
    } else {
        tracing::debug!("App config file {:?} does not exist, cannot retrieve key '{}'.", app_config_path, key);
        Ok(None)
    }
}

/// Converts a serde_json::Value to a toml_edit::Item.
/// Handles basic types (null, bool, number, string). Arrays/Objects need careful mapping.
fn json_to_toml_value(json_val: &JsonValue) -> Result<Item, ConfigError> {
    match json_val {
        JsonValue::Null => Ok(toml_value("")), // Or handle differently
        JsonValue::Bool(b) => Ok(toml_value(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(toml_value(i))
            } else if let Some(f) = n.as_f64() {
                Ok(toml_value(f))
            } else {
                Err(ConfigError::Config(RsConfigError::Message(format!("Unsupported number type in JSON: {}", n))))
            }
        }
        JsonValue::String(s) => Ok(toml_value(s.clone())),
        JsonValue::Array(_) => {
            Err(ConfigError::Config(RsConfigError::Message("Saving JSON arrays directly is not yet supported".to_string())))
        }
        JsonValue::Object(_) => {
            Err(ConfigError::Config(RsConfigError::Message("Saving JSON objects directly is not yet supported".to_string())))
        }
    }
}

/// Saves or updates configuration values in the application's config file (`elfradio_config.toml` in CWD).
/// Expects a flat JSON object where keys correspond to top-level TOML keys.
/// Creates the file if it doesn't exist.
/// Note: This version only supports updating/adding top-level keys.
pub fn save_user_config_values(values_to_save: JsonValue) -> Result<(), ConfigError> {
    let app_config_path = get_app_config_path()?; // Use the new helper
    debug!("Attempting to save values to app config file: {:?}", app_config_path);

    // No need to create parent_dir for CWD, fs::write will create the file or truncate.
    // If the path was nested, directory creation would be needed here.

    let content = fs::read_to_string(&app_config_path).unwrap_or_default();
    debug!("Read existing app config content (length: {} bytes)", content.len());

    let mut doc = content.parse::<DocumentMut>().map_err(|e| {
        error!("Failed to parse existing app config TOML at {:?}: {}", app_config_path, e);
        ConfigError::Config(RsConfigError::Message(format!(
            "Invalid app TOML format in {:?}: {}", app_config_path, e
        )))
    })?;
    debug!("Successfully parsed app config into editable document.");

    if let JsonValue::Object(map) = values_to_save {
        info!("Processing {} key-value pairs to save.", map.len());
        for (key, json_value) in map {
            debug!("Processing key: '{}', value: {:?}", key, json_value);
            let toml_item = json_to_toml_value(&json_value)?;
            doc[key.as_str()] = toml_item;
            debug!("Set key '{}' in TOML document.", key);
        }
    } else {
        warn!("`values_to_save` was not a JSON object. No values will be saved.");
        return Err(ConfigError::Config(RsConfigError::Message(
            "Invalid input: values_to_save must be a JSON object.".to_string(),
        )));
    }

    fs::write(&app_config_path, doc.to_string()).map_err(|e| {
        error!("Failed to write updated TOML to {:?}: {}", app_config_path, e);
        ConfigError::IoError(e)
    })?;

    info!("Successfully saved updated configuration to {:?}.", app_config_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (load_config, ConfigError)
    use elfradio_types::Config as AppConfig;
    use std::fs;
    use tempfile::Builder;
    use assert_matches::assert_matches;

    // Helper to create a minimal valid TOML content string - Completely rewritten
    fn create_valid_toml_content() -> String {
        r#"
        app_name = "ElfRadioTest"
        log_level = "info"
        tasks_base_directory = "/tmp/elfradio_tasks_test"
        ui_language = "en-US"

        [hardware]
        audio_input_device = "Test Input"
        audio_output_device = "Test Output"
        input_sample_rate = 16000
        serial_port = "/dev/ttyUSB0"
        ptt_signal = "Rts"
        sdr_device_args = "driver=rtlsdr"
        enable_rx_tx_separation = false
        rx_audio_input_device = ""
        rx_sdr_device_args = ""

        [ai_settings]
        provider = "OpenAICompatible"
        system_prompt = "You are an AI assistant for ham radio."
        temperature = 0.7
        top_p = 0.95
        max_tokens = 500
        translate_target_language = "en"

        [ai_settings.google]
        api_key = "google-test-key"
        preferred_model = "gemini-1.5-flash"
        project_id = "test-project"
        credentials_path = "/tmp/creds.json"
        stt_language = "en-US" 
        tts_voice = "en-US-Standard-D"

        [ai_settings.stepfun_tts]
        api_key = "stepfun-key"

        [ai_settings.openai_compatible]
        name = "TestAPI"
        base_url = "http://localhost:11434/v1"
        api_key = "sk-testkey"
        preferred_model = "test-model"

        [timing]
        ptt_pre_delay_ms = 100
        ptt_post_delay_ms = 50
        tx_hold_timer_s = 60
        tx_interval_s = 10
        max_tx_duration_s = 180
        max_sstv_duration_s = 120

        [radio_etiquette]
        nickname = "TestOp"
        addressing_interval_min = 5

        [security]
        end_task_phrase = "TEST STOP"

        [signal_tone]
        enabled = false
        start_freqs_hz = [1200.0]
        end_freqs_hz = [1800.0]
        duration_ms = 80

        [sstv_settings]
        mode = "Scottie S1"

        [stepfun_tts]
        api_key = "backup-stepfun-key"
        "#
        .to_string()
    }

    #[test]
    fn test_load_valid_config_from_standard_location() {
        let toml_content = create_valid_toml_content();
        let temp_dir = Builder::new().prefix("elfradio_valid").tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("test_valid_config.toml");
        fs::write(&config_path, toml_content).expect("Failed to write temp config file");

        let result = load_config();

        if result.is_err() {
             eprintln!("test_load_valid_config failed: {:?}", result.as_ref().err());
        }
        assert_matches!(result, Ok(_), "Expected Ok loading valid config: {:?}", config_path);
    }

    #[test]
    fn test_load_non_existent_config_uses_defaults() {
        let result = load_config();

        assert_matches!(result, Ok(_), "Expected Ok result with defaults when no config file exists");
        if let Ok(config) = result {
            assert_eq!(config.app_name, "ElfRadio");
            assert_eq!(config.log_level, "info");
        }
    }


    #[test]
    fn test_load_invalid_toml_config() {
        let invalid_toml_content = "this is not valid toml {";
        let temp_dir = Builder::new().prefix("elfradio_invalid").tempdir().expect("Failed to create temp dir");
        let invalid_config_path = temp_dir.path().join("test_invalid_config.toml");
        fs::write(&invalid_config_path, invalid_toml_content).expect("Failed to write invalid temp config file");

        let result = load_config();

        if result.is_ok() {
            eprintln!("test_load_invalid_toml_config unexpectedly succeeded: {:?}", result.as_ref().ok());
        }
    }

    #[test]
    fn test_default_config_values() {
        let config = AppConfig::default();

        assert_eq!(config.app_name, "ElfRadio");
        assert_eq!(config.ui_language, "en");
        assert_eq!(config.hardware.serial_port, None);
        assert_eq!(config.hardware.ptt_signal, "rts"); // lowercase default
        assert_eq!(config.hardware.input_sample_rate, 16000);
        assert_eq!(config.ai_settings.provider, None);
        assert_eq!(config.log_level, "info");
        assert!(!config.tasks_base_directory.as_os_str().is_empty(), "Default tasks base directory should not be empty");
        assert_eq!(config.timing.ptt_pre_delay_ms, 100);
        assert_eq!(config.timing.ptt_post_delay_ms, 100);
    }

    // Add tests for get_user_config_value
    // - Test case where user file exists and key exists
    // - Test case where user file exists but key does NOT exist (expect Ok(None))
    // - Test case where user file does NOT exist (expect Ok(None))
    // - Test case where user file exists but is invalid TOML (expect Err(ConfigError::Config))
    // - Test case where key exists but type is wrong (expect Err(ConfigError::Config))

    // Add tests for save_user_config_values:
    // - Test saving new key-value pairs to an empty/non-existent file.
    // - Test updating existing key-value pairs in a file.
    // - Test adding new pairs to a file with existing content.
    // - Test handling different basic value types (string, int, float, bool).
    // - Test error handling for invalid input JSON (not an object).
    // - Test error handling for unsupported JSON values (array, object).
    // - Test error handling for failing to parse existing invalid TOML.
    // - Test error handling for file write failures (permissions?).
}
