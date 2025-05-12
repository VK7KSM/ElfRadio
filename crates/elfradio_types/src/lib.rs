use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::Instant;
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use axum::extract::ws::Message;
use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use async_trait::async_trait;

// 1. TaskStatus 枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Idle,
    Running,
    Stopping,
}

// 2. TaskMode 枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskMode {
    GeneralCommunication,
    AirbandListening,
    SatelliteCommunication,
    EmergencyCommunication,
    MeshtasticGateway,
    SimulatedQsoPractice,
}

// 3. TaskInfo 结构体
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: Uuid,
    pub name: String,
    pub mode: TaskMode,
    pub start_time: Instant,
    pub task_dir: PathBuf,
    pub is_simulation: bool,
}

// 4. TxItem 枚举
#[derive(Debug, Clone)]
pub enum TxItem {
    ManualText { id: Uuid, text: String, priority: u8 },
    ManualVoice { id: Uuid, path: PathBuf, priority: u8 },
    AiReply { id: Uuid, text: String, priority: u8 },
    GeneratedVoice { id: Uuid, audio_data: Vec<f32>, priority: u8 },
    // 后续阶段添加其他变体，如 SSTV, CW 等
}

impl TxItem {
    fn priority(&self) -> u8 {
        match self {
            TxItem::ManualText { priority, .. } => *priority,
            TxItem::ManualVoice { priority, .. } => *priority,
            TxItem::AiReply { priority, .. } => *priority,
            TxItem::GeneratedVoice { priority, .. } => *priority,
        }
    }

    pub fn id(&self) -> Uuid {
        match self {
            TxItem::ManualText { id, .. } => *id,
            TxItem::ManualVoice { id, .. } => *id,
            TxItem::AiReply { id, .. } => *id,
            TxItem::GeneratedVoice { id, .. } => *id,
        }
    }
}

impl PartialEq for TxItem {
    fn eq(&self, other: &Self) -> bool {
        self.priority() == other.priority()
    }
}

impl Eq for TxItem {}

impl PartialOrd for TxItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TxItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority().cmp(&other.priority())
    }
}

// 5. Config 结构体框架 (Phase 1 版本) -> 更新为 V1.0 Rev5 详细定义

/// Hardware configuration settings.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HardwareConfig {
    /// Preferred audio input device name. None uses system default.
    pub audio_input_device: Option<String>,
    /// Preferred audio output device name. None uses system default.
    pub audio_output_device: Option<String>,
    /// Input audio sample rate in Hz (e.g., 16000, 48000).
    pub input_sample_rate: u32,
    /// Serial port for PTT/CAT control (e.g., "COM3" or "/dev/ttyUSB0").
    pub serial_port: Option<String>,
    /// PTT signal line ("rts" or "dtr").
    pub ptt_signal: String,
    /// SDR device arguments (e.g., "driver=rtlsdr"). (Phase 2+)
    pub sdr_device_args: Option<String>,
    /// Enable separate RX/TX hardware paths. (Phase 3+)
    pub enable_rx_tx_separation: bool,
    /// Specific audio input device for RX when separated. (Phase 3+)
    pub rx_audio_input_device: Option<String>,
    /// Specific SDR device arguments for RX when separated. (Phase 3+)
    pub rx_sdr_device_args: Option<String>,
}

// --- New AI Configuration Structs (V1.1.1 Rev1) ---

/// Defines the available AI service providers.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AiProvider {
    GoogleGemini,
    StepFunTTS, // Specifically for StepFun's TTS/Voice capabilities
    OpenAICompatible,
    // Add other specific providers later if needed
}

/// Defines the available auxiliary service providers.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AuxServiceProvider {
    Google,
    Aliyun,
    Baidu, // Placeholder for future
    // Add others as needed
}

/// Configuration specific to Google AI services (Gemini).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GoogleConfig {
    /// Google Cloud API Key (prefer this over project ID/credentials file for Gemini).
    pub api_key: Option<String>,
    /// Preferred Gemini model identifier.
    pub preferred_model: Option<String>, // e.g., "gemini-1.5-flash-latest"
    // Google Cloud Project ID and credentials path might be needed for other Google services (TTS/STT)
    // but are kept separate from the core Gemini LLM config for now.
    pub project_id: Option<String>,
    pub credentials_path: Option<PathBuf>,
    /// Default language code for Speech-to-Text (e.g., "en-US", "zh-CN").
    pub stt_language: Option<String>, // Made optional, can use a default
    /// Default voice name for Text-to-Speech (e.g., "en-US-Wavenet-D").
    pub tts_voice: Option<String>, // Made optional, can use a default
}

/// Configuration specific to Aliyun auxiliary services.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AliyunAuxCredentials {
    // These fields are primarily for config structure definition.
    // The actual values will be read by the client using get_user_config_value.
    // Mark them as optional in the config file.
    #[serde(default)]
    pub access_key_id: Option<String>,
    #[serde(default)]
    pub access_key_secret: Option<String>,
    #[serde(default)]
    pub app_key: Option<String>,
    // Add region_id etc. if needed by the client later
}

/// Configuration specific to the StepFun TTS service.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StepFunTtsConfig {
    /// API Key for StepFun TTS.
    #[serde(default)] // Make optional in TOML, read via get_user_config_value later if needed
    pub api_key: Option<String>,
    // Add other StepFun specific fields if any (e.g., preferred_voice)
}

/// Configuration specific to Baidu auxiliary services.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BaiduAuxConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub secret_key: Option<String>,
}

/// Configuration for OpenAI-compatible APIs (like StepFun Chat, DeepSeek Chat).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OpenAICompatibleConfig {
    /// User-defined name for this configuration (e.g., "StepFun Chat", "DeepSeek API").
    pub name: Option<String>,
    /// The base URL of the OpenAI-compatible API endpoint. Crucial.
    pub base_url: Option<String>,
    /// API Key for the service.
    pub api_key: Option<String>,
    /// Preferred model identifier for this service.
    pub preferred_model: Option<String>, // e.g., "step-1v-8k", "deepseek-chat"
}

/// Main AI configuration structure, supporting multiple providers.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AiConfig {
    /// The currently selected AI provider to use for primary LLM/TTS tasks.
    pub provider: Option<AiProvider>,

    /// Configuration details for Google AI services.
    pub google: Option<GoogleConfig>,
    /// Configuration details for StepFun TTS service.
    pub stepfun_tts: Option<StepFunTtsConfig>,
    /// Configuration details for a generic OpenAI-compatible service.
    /// Note: You might have multiple instances of this conceptually,
    /// but the config structure holds one active configuration.
    /// UI/Config logic would manage selecting which OpenAI compatible instance is active.
    pub openai_compatible: Option<OpenAICompatibleConfig>,

    // --- Common Parameters ---
    // These can potentially be overridden by provider-specific settings if needed,
    // but serve as general defaults.

    /// Default LLM temperature (sampling randomness). Typically 0.0 to 2.0.
    pub temperature: Option<f32>,
    /// Default LLM top-p (nucleus sampling probability mass). Typically 0.0 to 1.0.
    pub top_p: Option<f32>,
    /// Default maximum number of tokens to generate in an LLM completion.
    pub max_tokens: Option<u32>,

    /// Default system prompt to guide the LLM's behavior.
    pub system_prompt: Option<String>,
    /// Default target language code for translation (e.g., "en", "zh"). Used if translation needed.
    pub translate_target_language: Option<String>, // Made optional

    // Common TTS parameters (voice selection is handled by provider config or defaults)
    // pub default_tts_speed: Option<f32>, // Example: Add if needed later
    // pub default_tts_volume: Option<f32>, // Example: Add if needed later
}

/// Main auxiliary service configuration structure, supporting multiple providers.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AuxServiceConfig {
    /// Which provider to use for STT, TTS, and Translate.
    #[serde(default)] // Make provider optional in TOML, default handled by factory
    pub provider: Option<AuxServiceProvider>,

    /// Google specific settings placeholder (key read directly by client).
    #[serde(default)]
    pub google: GoogleConfig, // Reuse GoogleConfig for structure

    /// Aliyun specific settings placeholder.
    #[serde(default)]
    pub aliyun: AliyunAuxCredentials,

    /// Baidu specific settings placeholder.
    #[serde(default)]
    pub baidu: BaiduAuxConfig,
}

// --- End New AI Configuration Structs ---

/// Timing and delay configuration settings.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimingConfig {
    /// Delay before PTT activation (milliseconds).
    pub ptt_pre_delay_ms: u64,
    /// Delay after PTT deactivation (milliseconds).
    pub ptt_post_delay_ms: u64,
    /// Hold timer between transmissions (seconds). Prevents rapid back-and-forth.
    pub tx_hold_timer_s: u64,
    /// Minimum interval between automatic transmissions (seconds).
    pub tx_interval_s: u64,
    /// Maximum duration for a single continuous transmission (seconds).
    pub max_tx_duration_s: u64,
    /// Maximum duration specifically for SSTV transmissions (seconds).
    pub max_sstv_duration_s: u64,
}

/// Radio etiquette settings.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RadioEtiquetteConfig {
    /// Operator nickname/callsign used in automated messages.
    pub nickname: String,
    /// Minimum interval (minutes) between sending station identification or address.
    pub addressing_interval_min: u32,
}

/// Security-related settings.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecurityConfig {
    /// Voice command phrase to immediately stop the current task.
    pub end_task_phrase: String,
}

/// Configuration for start/end signal tones.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalToneConfig {
    /// Enable sending signal tones before/after transmissions.
    pub enabled: bool,
    /// Frequencies (Hz) for the start tone sequence.
    pub start_freqs_hz: Vec<f32>,
    /// Frequencies (Hz) for the end tone sequence.
    pub end_freqs_hz: Vec<f32>,
    /// Duration (milliseconds) for each tone segment.
    pub duration_ms: u64,
}

/// SSTV (Slow-Scan Television) specific settings.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SstvConfig {
    /// Default SSTV mode (e.g., "Martin M1", "Scottie S1").
    pub mode: String,
}

/// Network configuration settings.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    /// API 服务器监听地址
    pub listen_address: Option<String>,
    /// API 服务器监听端口
    pub listen_port: Option<u16>,
}

/// Main application configuration structure.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// Application name (mostly for logging/identification).
    pub app_name: String,
    /// Logging level (e.g., "debug", "info", "warn").
    pub log_level: String,
    /// Base directory for storing task-related data.
    pub tasks_base_directory: PathBuf,
    /// User interface language code (e.g., "en", "zh").
    pub ui_language: String,
    /// Hardware settings.
    pub hardware: HardwareConfig,
    /// AI service settings. **(Updated)**
    pub ai_settings: AiConfig,
    /// Auxiliary service settings for translation, TTS, STT.
    #[serde(default)]
    pub aux_service_settings: AuxServiceConfig,
    /// Timing settings.
    pub timing: TimingConfig,
    /// Radio etiquette settings.
    pub radio_etiquette: RadioEtiquetteConfig,
    /// Security settings.
    pub security: SecurityConfig,
    /// Signal tone settings.
    pub signal_tone: SignalToneConfig,
    /// SSTV settings.
    pub sstv_settings: SstvConfig,
    /// Network configuration
    pub network: Option<NetworkConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            app_name: "ElfRadio".to_string(),
            log_level: "info".to_string(),
            tasks_base_directory: PathBuf::from("./elfradio_tasks"), // Example path
            ui_language: "en".to_string(),
            hardware: HardwareConfig {
                audio_input_device: None,
                audio_output_device: None,
                input_sample_rate: 16000,
                serial_port: None,
                ptt_signal: "rts".to_string(),
                sdr_device_args: None,
                enable_rx_tx_separation: false,
                rx_audio_input_device: None,
                rx_sdr_device_args: None,
            },
            // Updated to use AiConfig::default() and explicitly set provider to None
            ai_settings: AiConfig {
                provider: None, // 明确设置为 None
                google: None, // 保留为 None 或按需使用 Some(GoogleConfig::default())
                stepfun_tts: None, // 保留为 None 或按需使用 Some(StepFunTtsConfig::default())
                openai_compatible: None, // 保留为 None 或按需使用 Some(OpenAICompatibleConfig::default())
                // 保留或设置其他 ai_settings 字段的默认值
                temperature: Some(0.7), // 示例：保留默认值
                max_tokens: Some(1024), // 示例：保留默认值
                ..Default::default() // 使用 AiConfig 的其他默认值
            },
            // Initialize new aux service settings
            aux_service_settings: AuxServiceConfig::default(),
            timing: TimingConfig {
                ptt_pre_delay_ms: 100,
                ptt_post_delay_ms: 100,
                tx_hold_timer_s: 5,
                tx_interval_s: 60, // Default interval 1 minute
                max_tx_duration_s: 180, // Default max TX 3 minutes
                max_sstv_duration_s: 180, // Default max SSTV 3 minutes
            },
            radio_etiquette: RadioEtiquetteConfig {
                nickname: "ElfRadio Operator".to_string(),
                addressing_interval_min: 10, // Address every 10 minutes
            },
            security: SecurityConfig {
                end_task_phrase: "STOP TASK NOW".to_string(), // Example phrase
            },
            signal_tone: SignalToneConfig {
                enabled: false, // Disabled by default
                start_freqs_hz: vec![1000.0, 1500.0], // Example tones
                end_freqs_hz: vec![1500.0, 1000.0], // Example tones
                duration_ms: 100, // 100ms per tone
            },
            sstv_settings: SstvConfig {
                mode: "Martin M1".to_string(), // Default SSTV mode
            },
            network: Some(NetworkConfig {
                listen_address: Some("0.0.0.0".to_string()),
                listen_port: Some(5900),
            }),
        }
    }
}

/// Type alias for the map holding client WebSocket senders.
/// Key: Unique client ID.
/// Value: Sender channel to forward messages to the client's WebSocket task.
pub type ClientMap = Arc<Mutex<HashMap<Uuid, mpsc::UnboundedSender<Result<Message, axum::Error>>>>>;

/// Defines which serial port signal line to use for PTT control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttSignal {
    Rts,
    Dtr,
}

/// Represents messages sent from the audio input stream handler.
#[derive(Debug, Clone)] // Clone is needed if the sender might be cloned
pub enum AudioMessage {
    /// A chunk of raw audio samples (f32 format).
    Data(Vec<f32>),
    /// The calculated Root Mean Square (RMS) value for a chunk.
    Rms(f32),
    /// Indicates an error occurred within the audio callback.
    Error(String), // Optional: For reporting errors from the callback
}

// Define the sender type alias (assuming f32 samples for output)
pub type AudioOutputSender = mpsc::UnboundedSender<Vec<f32>>;

// Add FromStr impl for PttSignal if not already present
#[derive(Debug, Error)]
#[error("Invalid PTT signal type: {0}")]
pub struct PttSignalParseError(String);

impl FromStr for PttSignal {
    type Err = PttSignalParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rts" => Ok(PttSignal::Rts),
            "dtr" => Ok(PttSignal::Dtr),
            _ => Err(PttSignalParseError(format!("Unknown PTT signal type: {}", s))),
        }
    }
}

// --- Added Log Entry Definitions ---

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum LogDirection {
    Incoming,
    Outgoing,
    Internal, // For system messages or internal events
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum LogContentType {
    Text,
    Audio, // Represents a path to an audio file
    Status, // For logging start/end events, status changes etc.
    // Add other types later like Image(SSTV), Error etc.
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>, // Precise timestamp
    pub direction: LogDirection,
    pub content_type: LogContentType,
    pub content: String, // For Text, or File path for Audio/Image
    // Optional: Add source/destination info later if needed
    // Optional: Add unique event ID (Uuid)?
}

/// WebSocket Message Types
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")] // Use tag/content for easy frontend parsing
pub enum WebSocketMessage {
    Log(LogEntry), // 包装日志条目
    // 之后可以添加其他消息类型，例如:
    // TaskStatusUpdate { status: TaskStatus, task_id: Option<Uuid>, task_mode: Option<TaskMode> },
    // BackendStatus { status: String }, // 例如 "OK", "Error", "Starting"
}

/// Shared Error type for AI operations across elfradio crates.
#[derive(Error, Debug, Clone)] // Added Clone derive
pub enum AiError {
    #[error("Configuration error: {0}")]
    Config(String), // For config read/parse errors
    #[error("Audio processing error: {0}")]
    Audio(String),
    #[error("Unknown AI error")]
    Unknown, // Keep for truly unknown cases
    #[error("AI Client Error: {0}")]
    ClientError(String), // General client-side setup/config errors
    #[error("API Error (Status: {status}): {message}")]
    ApiError { status: u16, message: String }, // Errors reported by the AI service API
    #[error("Request Error: {0}")]
    RequestError(String), // Errors during HTTP request building or sending
    #[error("Response Parse Error: {0}")]
    ResponseParseError(String), // Errors parsing the API response (e.g., invalid JSON)
    #[error("Audio Decoding Error: {0}")]
    AudioDecodingError(String), // Errors decoding TTS audio bytes (e.g., invalid format)
    #[error("Operation Not Supported: {0}")]
    NotSupported(String), // Feature requested is not supported by the provider
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("AI provider not specified in configuration")]
    ProviderNotSpecified,
    // Example, mirroring elfradio_aux_client::AiError:
    #[error("Authentication Error: {0}")]
    AuthenticationError(String),
}

/// Request body structure for updating configuration via API.
#[derive(Deserialize, Debug)]
pub struct UpdateConfigRequest {
    #[serde(flatten)]
    pub updates: HashMap<String, JsonValue>,
}

// Add the test module at the end of the file
#[cfg(test)]
mod tests {
    use super::TxItem;
    use std::path::PathBuf;
    use uuid::Uuid;

    // 添加此辅助函数
    fn get_id(item: &TxItem) -> Uuid {
        match item {
            TxItem::ManualText { id, .. } => *id,
            TxItem::ManualVoice { id, .. } => *id,
            TxItem::AiReply { id, .. } => *id,
            TxItem::GeneratedVoice { id, .. } => *id, // 确保包含所有变体
        }
    }

    #[test]
    fn test_tx_item_priority_order() {
        // 1. Create TxItem instances with varying priorities
        let ai_low = TxItem::AiReply {
            id: Uuid::new_v4(),
            text: "ai low priority".into(),
            priority: 1,
        };
        let manual_medium = TxItem::ManualText {
            id: Uuid::new_v4(),
            text: "manual medium priority".into(),
            priority: 5,
        };
        let manual_high_voice = TxItem::ManualVoice {
            id: Uuid::new_v4(),
            path: PathBuf::new(), // Dummy path
            priority: 10,
        };
        let ai_high_reply = TxItem::AiReply {
            id: Uuid::new_v4(),
            text: "ai high priority".into(),
            priority: 10, // Same high priority
        };

        // 2. Create a vector in an unsorted order
        let mut items = vec![
            manual_medium.clone(), // Prio 5
            manual_high_voice.clone(), // Prio 10
            ai_low.clone(), // Prio 1
            ai_high_reply.clone(), // Prio 10
        ];

        // 3. Sort the vector using the Ord implementation
        items.sort(); // Standard sort uses Ord, sorts ascending (lowest first)

        // 4. Assert the sorted order based on priority
        // Lowest priority should be first
        assert_eq!(items[0].priority(), 1, "Item with lowest priority (1) should be first.");
        assert_eq!(get_id(&items[0]), get_id(&ai_low)); // 使用 get_id 替代 id() 方法

        // Medium priority should be next
        assert_eq!(items[1].priority(), 5, "Item with medium priority (5) should be second.");
        assert_eq!(get_id(&items[1]), get_id(&manual_medium));

        // Highest priority items should be last.
        // Their internal order (manual_high_voice vs ai_high_reply) might be
        // stable or unstable depending on sort and PartialOrd tie-breaking,
        // but they must come after priority 5.
        assert_eq!(items[2].priority(), 10, "First item with highest priority (10) should be third.");
        assert_eq!(items[3].priority(), 10, "Second item with highest priority (10) should be fourth.");

        // Verify the two high-priority items are present at the end, regardless of their internal order
        let high_priority_ids: Vec<Uuid> = items[2..].iter().map(|item| get_id(item)).collect();
        assert!(high_priority_ids.contains(&get_id(&manual_high_voice)));
        assert!(high_priority_ids.contains(&get_id(&ai_high_reply)));

        println!("Sorted items by priority: {:?}", items.iter().map(|i| i.priority()).collect::<Vec<_>>());
    }
}

// --- Frontend-Safe AI Configuration ---

#[derive(Serialize, Debug, Clone, Default)]
pub struct FrontendAiProviderDetails {
    // 只包含非敏感信息
    pub name: Option<String>, // For OpenAICompatible
    pub preferred_model: Option<String>,
    // Google specific non-sensitive fields (excluding api_key, credentials_path)
    pub project_id: Option<String>,
    pub stt_language: Option<String>,
    pub tts_voice: Option<String>,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct FrontendAiConfig {
    pub provider: Option<AiProvider>,
    // Use generic details struct to hold non-sensitive parts of ANY provider
    pub details: Option<FrontendAiProviderDetails>,
    // Common parameters are generally safe
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    pub translate_target_language: Option<String>,
}

// --- Main Frontend Configuration Struct ---

#[derive(Serialize, Debug, Clone)]
pub struct FrontendConfig {
    // --- General Settings ---
    pub app_name: String,
    pub log_level: String,
    pub ui_language: String,
    pub tasks_base_directory: PathBuf, // Base directory path might be okay

    // --- AI Settings (Safe Version) ---
    pub ai_settings: FrontendAiConfig,

    // --- Include other *safe* configurations ---
    // Decide carefully which fields to expose

    // Example: Exposing most hardware settings might be okay, except specific SDR args?
    pub hardware: HardwareConfig, // Assuming HardwareConfig itself is currently safe

    // Example: Timing is likely safe
    pub timing: TimingConfig,

    // Example: Etiquette is likely safe
    pub radio_etiquette: RadioEtiquetteConfig,

    // Example: Security - Expose only non-sensitive parts if needed, or omit
    // pub security: SecurityConfig, // Omitting end_task_phrase for now

    // Example: Signal Tone settings are likely safe
    pub signal_tone: SignalToneConfig,

    // Example: SSTV mode is likely safe
    pub sstv_settings: SstvConfig,

    // Example: Network settings (Port/Address) might be useful for frontend
    pub network: Option<NetworkConfig>,
}

impl From<&Config> for FrontendConfig {
    fn from(config: &Config) -> Self {
        // Manually map safe fields from the full Config to FrontendConfig

        // Create FrontendAiProviderDetails based on the selected provider
        let frontend_ai_details = match config.ai_settings.provider {
            Some(AiProvider::GoogleGemini) => config.ai_settings.google.as_ref().map(|g| FrontendAiProviderDetails {
                preferred_model: g.preferred_model.clone(),
                project_id: g.project_id.clone(),
                stt_language: g.stt_language.clone(),
                tts_voice: g.tts_voice.clone(),
                ..Default::default() // Initialize other fields to default
            }),
            Some(AiProvider::StepFunTTS) => config.ai_settings.stepfun_tts.as_ref().map(|_s| FrontendAiProviderDetails {
                // StepFun TTS might not have other non-sensitive details to show here
                 ..Default::default()
            }),
             // Also handle the separate top-level stepfun_tts field if it exists and is used
             // else if let Some(s) = &config.stepfun_tts { // Check the separate field
             //      Some(FrontendAiProviderDetails { ..Default::default() })
             // }
            Some(AiProvider::OpenAICompatible) => config.ai_settings.openai_compatible.as_ref().map(|o| FrontendAiProviderDetails {
                name: o.name.clone(),
                preferred_model: o.preferred_model.clone(),
                 ..Default::default()
            }),
            None => None,
        };


        FrontendConfig {
            app_name: config.app_name.clone(),
            log_level: config.log_level.clone(),
            ui_language: config.ui_language.clone(),
            tasks_base_directory: config.tasks_base_directory.clone(),

            ai_settings: FrontendAiConfig {
                provider: config.ai_settings.provider.clone(),
                details: frontend_ai_details, // Use the mapped safe details
                temperature: config.ai_settings.temperature,
                top_p: config.ai_settings.top_p,
                max_tokens: config.ai_settings.max_tokens,
                system_prompt: config.ai_settings.system_prompt.clone(),
                translate_target_language: config.ai_settings.translate_target_language.clone(),
            },

            // Clone safe sub-structs directly (assuming they are safe as defined)
            hardware: config.hardware.clone(),
            timing: config.timing.clone(),
            radio_etiquette: config.radio_etiquette.clone(),
            signal_tone: config.signal_tone.clone(),
            sstv_settings: config.sstv_settings.clone(),
            network: config.network.clone(),
            // Omit sensitive structs like `security` unless specific fields are mapped
        }
    }
}

/// Defines the interface for auxiliary services like translation, TTS, and STT.
#[async_trait]
pub trait AuxServiceClient: Send + Sync {
    /// Translates text from one language to another.
    ///
    /// # Arguments
    /// * `text` - The text to translate.
    /// * `target_language` - The language code to translate to (e.g., "en", "zh").
    /// * `source_language` - Optional source language code. If None, the service will attempt to detect it.
    ///
    /// # Returns
    /// A `Result` containing the translated text as a `String` on success,
    /// or an `AiError` on failure.
    async fn translate(&self, text: &str, target_language: &str, source_language: Option<&str>) -> Result<String, AiError>;

    /// Converts text to speech audio.
    ///
    /// # Arguments
    /// * `text` - The text to synthesize.
    /// * `language_code` - The language code (e.g., "en-US", "zh-CN").
    /// * `voice_name` - Optional voice identifier. If None, a default voice will be used.
    ///
    /// # Returns
    /// A `Result` containing the raw audio data as `Vec<u8>` on success,
    /// or an `AiError` on failure.
    async fn text_to_speech(&self, text: &str, language_code: &str, voice_name: Option<&str>) -> Result<Vec<u8>, AiError>;

    /// Converts speech audio to text.
    ///
    /// # Arguments
    /// * `audio_data` - Raw audio data bytes.
    /// * `sample_rate_hertz` - The sample rate of the audio in Hz (e.g., 16000, 48000).
    /// * `language_code` - The language code (e.g., "en-US", "zh-CN").
    ///
    /// # Returns
    /// A `Result` containing the transcribed text as a `String` on success,
    /// or an `AiError` on failure.
    async fn speech_to_text(&self, audio_data: &[u8], sample_rate_hertz: u32, language_code: &str) -> Result<String, AiError>;
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)] // Added PartialEq, Eq for potential use in tests
pub struct ChatMessage {
    pub role: String, // e.g., "user", "assistant", "system"
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)] // Added PartialEq, Default
pub struct ChatParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    // Add other common LLM parameters as needed, ensuring they are optional
}

/// Request body for the /api/test/llm endpoint.
#[derive(Deserialize, Debug, Clone)] // Added Clone
pub struct TestLlmRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(default)] // Use default for Option<ChatParams> if not provided
    pub params: Option<ChatParams>,
}

// --- Test API Request Structs ---

/// Request body for the TTS test endpoint.
#[derive(Deserialize, Debug, Clone)]
pub struct TestTtsRequest {
    pub text: String,
    pub language_code: String, // e.g., "zh-CN", "en-US"
    pub voice_name: Option<String>, // e.g., "Aiyue"
}

/// Request body for the STT test endpoint.
#[derive(Deserialize, Debug, Clone)]
pub struct TestSttRequest {
    /// Base64 encoded audio data.
    pub audio_base64: String,
    /// Sample rate of the audio in Hertz (e.g., 16000).
    pub sample_rate_hertz: u32,
    /// Language code for STT (e.g., "zh-CN", "en-US").
    pub language_code: String,
}
