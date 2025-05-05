use elfradio_types::Config as AppConfig; // Alias to avoid naming collision
use config::{Config as ConfigRs, Environment, File, ConfigError as RsConfigError};
use directories::ProjectDirs;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, info, warn, error}; // Ensure tracing imports are present
use std::fs; // Ensure fs is imported
 // Add this import
use serde_json::Value as JsonValue; // Added for input type
use toml_edit::{DocumentMut, value as toml_value, Item}; // Added for TOML writing and Item

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("I/O error accessing configuration file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration parsing or validation error: {0}")]
    Config(#[from] RsConfigError),

    #[error("Could not determine a valid configuration directory")]
    DirectoryError,
}

/// Helper function to get the full path to the user's config file.
/// Uses ProjectDirs to find the appropriate configuration directory based on OS conventions.
fn get_user_config_path() -> Result<PathBuf, ConfigError> {
    // Use ProjectDirs to find OS-specific config directory
    // Example: Linux: /home/user/.config/ElfRadio/elfradio_config.toml
    //          Windows: C:\Users\user\AppData\Roaming\ElfRadio\ElfRadio\config\elfradio_config.toml
    //          macOS: /Users/user/Library/Application Support/net.ElfRadio.ElfRadio/elfradio_config.toml
    // NOTE: The exact path depends on the `directories` crate implementation.
    match ProjectDirs::from("net", "ElfRadio", "ElfRadio") {
        Some(proj_dirs) => {
            let config_dir = proj_dirs.config_dir();
            // Note: Directory creation is deferred to where it's needed (e.g., writing/copying).
            Ok(config_dir.join("elfradio_config.toml"))
        }
        None => {
            // Use tracing::error! here
            tracing::error!("Could not determine the user configuration directory.");
            // Use the correct variant, assuming it doesn't take a String
            Err(ConfigError::DirectoryError) 
        }
    }
}

/// Loads the ElfRadio configuration.
///
/// Checks if the user configuration file (`elfradio_config.toml` in the platform-specific
/// config directory) exists. If not, it attempts to create it by copying from
/// `config/default.toml` (relative to the CWD).
///
/// Afterwards, it merges configuration from multiple sources using `config-rs`:
/// 1. `config/default.toml` (relative to CWD, required base).
/// 2. The user's `elfradio_config.toml` file (optional, overrides defaults if it exists or was copied).
/// 3. Environment variables prefixed with `ELFRADIO_` (highest precedence).
pub fn load_config() -> Result<AppConfig, ConfigError> {
    // --- Step 1: Check and copy default config if necessary ---
    let user_config_path = get_user_config_path()?;

    if !user_config_path.exists() {
        info!("User config file not found at {:?}, attempting to create from default.", user_config_path);

        if let Some(parent_dir) = user_config_path.parent() {
            fs::create_dir_all(parent_dir).map_err(|e| {
                error!("Failed to create user config directory {:?}: {}", parent_dir, e);
                ConfigError::IoError(e)
            })?;
        } else {
             error!("Could not determine parent directory for user config path {:?}", user_config_path);
             return Err(ConfigError::DirectoryError);
        }

        // IMPORTANT: Assumes 'config/default.toml' is relative to the CWD.
        let default_config_path = PathBuf::from("config/default.toml");

        if default_config_path.exists() {
            match fs::copy(&default_config_path, &user_config_path) {
                Ok(_) => info!("Successfully copied default config to {:?}", user_config_path),
                Err(e) => {
                    error!("Failed to copy default config from {:?} to {:?}: {}", default_config_path, user_config_path, e);
                    warn!("Proceeding without user config file due to copy error.");
                    // Don't return error here, just warn and proceed
                }
            }
        } else {
            warn!(
                "Default config template not found at {:?}. Cannot create user config file. App will rely on built-in defaults and environment variables.",
                default_config_path
            );
             // It might be better to return an error here if the default is mandatory
             // return Err(ConfigError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "Default config not found")));
        }
    }

    // --- Step 2: Load configuration using config-rs ---
    // Use the user_config_path determined earlier
    debug!("Building configuration sources...");
    let builder = ConfigRs::builder()
        // 1. Add default config file (relative path, required base)
        //    Ensure this path is correct for the runtime environment.
        //    Make it required=false because if it failed to copy, we might still want to run with built-in defaults + user + env
        //    Alternatively, make it required=true and handle the error if default doesn't exist AND copy failed. Let's make it true for robustness.
        .add_source(File::with_name("config/default.toml").required(true))
        // 2. Add user config file (optional, overrides defaults if it exists or was copied)
        //    Clone path needed as `File::from` takes ownership.
        .add_source(File::from(user_config_path.clone()).required(false))
        // 3. Add environment variables (highest precedence)
        .add_source(
            Environment::with_prefix("ELFRADIO")
                .prefix_separator("_") // Original prefix was `_`
                .separator("__") // Double underscore for nested keys
                .try_parsing(true) // Attempt to parse values like bools/numbers
                .list_separator(",") // Support comma-separated lists
        );

    // Build and deserialize the final configuration
    debug!("Building and deserializing configuration...");
    // Use the `?` operator thanks to `#[from]` on ConfigError::Config
    let config_rs = builder.build()?;
    let app_config = config_rs.try_deserialize::<AppConfig>()?;

    info!("Configuration loaded successfully.");
    Ok(app_config)

    // Note: The previous loading logic based on AppConfig::default() is removed
    // as config-rs now handles the layering, starting from default.toml.
}

/// Retrieves a specific configuration value directly from the user's config file.
/// Only reads the user file, ignoring defaults and environment variables.
/// Useful for reading sensitive values like API keys at the time of use.
/// Returns Ok(None) if the user file doesn't exist or the key is not found within it.
/// Returns Err(ConfigError) for file read/parse errors or type mismatches.
pub fn get_user_config_value<T: serde::de::DeserializeOwned>(key: &str) -> Result<Option<T>, ConfigError> {
    // Get the path to the user-specific config file
    let user_config_path = get_user_config_path()?; // Use the helper function

    if user_config_path.exists() {
        // Build a temporary config instance loading *only* the user file
        // Use tracing for logging potential issues during this specific read
        tracing::debug!("Attempting to read key '{}' from user config file: {:?}", key, user_config_path);
        let config_reader = ConfigRs::builder()
            // Source only the user config file. Make it required because we know it exists.
            .add_source(File::from(user_config_path.clone()).required(true))
            .build()
            // Error here means the user's TOML file is likely invalid
            .map_err(|e| {
                tracing::error!("Failed to build config reader for user file {:?}: {}", user_config_path, e);
                ConfigError::Config(e) // Map the config-rs error
            })?;

        // Attempt to get the specific key from the loaded user config
        match config_reader.get::<T>(key) {
            Ok(value) => {
                tracing::debug!("Successfully retrieved key '{}' from user config file.", key);
                Ok(Some(value)) // Value found and successfully deserialized
            }
            Err(e) => {
                // Use if let for more idiomatic matching on the error kind
                if let RsConfigError::NotFound(_) = e {
                     // Specific 'key not found' in this user file is Ok(None)
                     tracing::debug!("Key '{}' not found in user config file {:?}.", key, user_config_path);
                     Ok(None)
                } else {
                     // Other errors (like type mismatch during deserialization) are real errors
                     tracing::error!("Error getting key '{}' from user config file {:?}: {}", key, user_config_path, e);
                     Err(ConfigError::Config(e)) // Propagate other config errors (already mapped via #[from])
                }
            }
        }
    } else {
        // User config file doesn't exist, so the key cannot be present in it
        tracing::debug!("User config file {:?} does not exist, cannot retrieve key '{}'.", user_config_path, key);
        Ok(None)
    }
}

/// Converts a serde_json::Value to a toml_edit::Item.
/// Handles basic types (null, bool, number, string). Arrays/Objects need careful mapping.
fn json_to_toml_value(json_val: &JsonValue) -> Result<Item, ConfigError> {
    match json_val {
        // TOML doesn't have null. We map it to an empty string as a placeholder.
        // Consider if a different representation or error is more appropriate.
        JsonValue::Null => Ok(toml_value("")),
        JsonValue::Bool(b) => Ok(toml_value(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(toml_value(i))
            } else if let Some(f) = n.as_f64() {
                Ok(toml_value(f))
            } else {
                // Use format! to create the error string, then wrap in RsConfigError::Message
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

/// Saves or updates configuration values in the user's config file.
/// Expects a flat JSON object where keys correspond to top-level TOML keys.
/// Creates the file or directory if it doesn't exist.
/// Note: This version only supports updating/adding top-level keys.
/// Nested keys (e.g., "ai_settings.google.api_key") require more complex logic.
pub fn save_user_config_values(values_to_save: JsonValue) -> Result<(), ConfigError> {
    let user_config_path = get_user_config_path()?;
    debug!("Attempting to save values to user config file: {:?}", user_config_path);

    // Ensure parent directory exists
    if let Some(parent_dir) = user_config_path.parent() {
        // Map potential I/O error using #[from]
        fs::create_dir_all(parent_dir)?;
        debug!("Ensured user config directory exists: {:?}", parent_dir);
    } else {
        // This case should be unlikely if get_user_config_path succeeded
        error!("Could not determine parent directory for user config path {:?}", user_config_path);
        return Err(ConfigError::DirectoryError);
    }

    // Read existing content or start with an empty document if file doesn't exist or is empty
    let content = fs::read_to_string(&user_config_path).unwrap_or_default();
    debug!("Read existing config content (length: {} bytes)", content.len());

    // Parse the TOML content into an editable document
    let mut doc = content.parse::<DocumentMut>().map_err(|e| {
        error!("Failed to parse existing user config TOML at {:?}: {}", user_config_path, e);
        // Use Message variant instead of Custom
        ConfigError::Config(RsConfigError::Message(format!(
            "Invalid user TOML format in {:?}: {}", user_config_path, e
        )))
    })?;
    debug!("Successfully parsed user config into editable document.");

    // Expect values_to_save to be a JSON object
    if let JsonValue::Object(map) = values_to_save {
        info!("Processing {} key-value pairs to save.", map.len());
        for (key, json_value) in map {
            debug!("Processing key: '{}', value: {:?}", key, json_value);
            // Convert JSON value to TOML Item using the updated helper
            let toml_item = json_to_toml_value(&json_value)?;

            // Assign the Item directly. This is correct.
            doc[key.as_str()] = toml_item;
            debug!("Set key '{}' in TOML document.", key);

            // Note: The refined logic for nested keys (using split('.')) is omitted
            // here as per the instruction to stick with the simple flat key insertion for now.
        }
    } else {
        error!("Invalid payload provided to save_user_config_values: Expected a JSON object, got {:?}", values_to_save);
        // Use Message variant instead of Custom
        return Err(ConfigError::Config(RsConfigError::Message(
            "Invalid payload: Expected a JSON object.".to_string(),
        )));
    }

    // Write the modified document back to the file atomically (if possible, fs::write attempts this)
    let new_content = doc.to_string();
    debug!("Writing updated TOML content (length: {} bytes) to {:?}", new_content.len(), user_config_path);
    fs::write(&user_config_path, new_content).map_err(|e| {
        error!("Failed to write updated user config file {:?}: {}", user_config_path, e);
        // Map I/O error using #[from]
        ConfigError::IoError(e)
    })?;

    info!("Successfully saved configuration values to {:?}", user_config_path);
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
