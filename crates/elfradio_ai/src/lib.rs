// Example, adjust as needed
// use tracing::error; // Keep only error as it's used
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use elfradio_types::AiError;

// Declare the new submodules
mod openai_compatible;
// DELETE: mod google; // Added google module
pub mod stepfun; // Add this line

// Publicly export the client structs from the submodules
pub use openai_compatible::OpenAICompatibleClient;
// DELETE: pub use google::GoogleAiClient; // Added GoogleAiClient export
pub use stepfun::StepFunTtsClient; // Optionally re-export

/// 辅助函数：将 serde_json::Error 转换为 AiError
pub fn json_error_to_ai_error(err: serde_json::Error) -> AiError {
        AiError::ResponseParseError(format!("JSON parsing failed: {}", err))
}

/// Represents a single message in a chat conversation history.
/// Mirrors the structure commonly used by OpenAI, Gemini, StepFun, etc.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    /// The role of the message author ("system", "user", or "assistant").
    pub role: String,
    /// The content of the message.
    pub content: String,
}

/// Parameters for requesting a chat completion.
#[derive(Debug, Clone, Default)]
pub struct ChatParams {
    /// The specific model identifier to use for the completion.
    /// If None, will attempt to use the provider's configured preferred_model.
    pub model: Option<String>,
    /// Sampling temperature. Controls randomness. Lower values make output more deterministic.
    pub temperature: Option<f32>, // Typically 0.0 to 2.0
    /// Nucleus sampling parameter. Controls diversity.
    pub top_p: Option<f32>, // Typically 0.0 to 1.0
    /// The maximum number of tokens to generate in the completion.
    pub max_tokens: Option<u32>,
    pub timeout_seconds: Option<u64>, // 添加超时选项
    // Add other potential parameters like stop sequences, presence_penalty etc. if needed
}

/// Parameters for requesting text-to-speech synthesis.
#[derive(Debug, Clone)]
pub struct TtsParams {
    /// Identifier for the desired voice. Provider-specific.
    pub voice_id: String,
    /// Optional language code (e.g., "en-US", "zh-CN"). Often inferred from voice.
    pub language_code: Option<String>,
    /// Speech speed/rate multiplier (e.g., 1.0 for normal).
    pub speed: Option<f32>,
    /// Speech volume/gain (e.g., 1.0 for normal). Provider support varies.
    pub volume: Option<f32>,
    /// Desired output audio format (e.g., "wav", "mp3", "pcm_f32le"). Provider-specific.
    pub output_format: String,
}

/// Parameters for requesting speech-to-text transcription.
#[derive(Debug, Clone)]
pub struct SttParams {
    /// The specific model identifier to use for transcription.
    pub model: Option<String>, // Some providers might have specific models
    /// The language of the audio data (e.g., "en-US", "zh-CN"). Crucial.
    pub language_code: String,
    /// The sample rate of the audio data in Hz (e.g., 16000, 48000).
    pub sample_rate: u32,
    /// The audio encoding format (e.g., "LINEAR16", "FLAC", "MP3"). Provider-specific.
    pub audio_format: String,
    // Add other potential parameters like punctuation hints, diarization etc. if needed
}

/// Defines the core asynchronous interface for interacting with AI services.
///
/// Implementations of this trait will handle communication with specific AI providers
/// (Google Gemini, StepFun, OpenAI-compatible APIs, etc.).
///
/// Must be implemented with `Send + Sync` to allow safe sharing across threads.
#[async_trait]
pub trait AiClient: Send + Sync {
    /// Generates a chat completion based on the provided message history and parameters.
    ///
    /// # Arguments
    /// * `messages` - A vector of `ChatMessage` structs representing the conversation history.
    /// * `params` - A reference to `ChatParams` containing model and generation settings.
    ///
    /// # Returns
    /// A `Result` containing the assistant's reply as a `String` on success,
    /// or an `AiError` on failure.
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        params: &ChatParams,
    ) -> Result<String, AiError>;

    /// Synthesizes speech audio from the provided text using specified parameters.
    ///
    /// # Arguments
    /// * `text` - The text content to synthesize.
    /// * `params` - A reference to `TtsParams` containing voice, format, and other settings.
    ///
    /// # Returns
    /// A `Result` containing the raw audio data as `Vec<u8>` on success
    /// (format determined by `params.output_format`), or an `AiError` on failure.
    async fn text_to_speech(
        &self,
        text: &str,
        params: &TtsParams,
    ) -> Result<Vec<u8>, AiError>;

    /// Transcribes speech audio into text using specified parameters.
    ///
    /// # Arguments
    /// * `audio_data` - Raw audio data bytes.
    /// * `params` - A reference to `SttParams` containing language, format, and model settings.
    ///
    /// # Returns
    /// A `Result` containing the transcribed text as a `String` on success,
    /// or an `AiError` on failure.
    async fn speech_to_text(
        &self,
        audio_data: &[u8], // Use slice for audio data
        params: &SttParams,
    ) -> Result<String, AiError>;

    /// (Optional) Lists the models available from the AI provider that are compatible
    /// with the client's configured purpose (e.g., chat models, TTS models).
    ///
    /// # Returns
    /// A `Result` containing a vector of model identifier strings on success,
    /// or an `AiError` (often `AiError::NotSupported`) on failure or if not implemented.
    async fn list_models(&self) -> Result<Vec<String>, AiError> {
        // Default implementation indicates the feature is not supported.
        // Implementations can override this if the provider API supports model listing.
        Err(AiError::NotSupported(
            "Listing models not supported by this provider implementation.".to_string(),
        ))
        // Alternatively, return Ok(Vec::new()) if preferred for unsupported cases.
        // Ok(Vec::new())
    }

    // Potential future methods:
    // async fn translate(&self, text: &str, params: &TranslateParams) -> Result<String, AiError>;
}

// Example of how you might use the trait object later:
// async fn use_ai_client(client: Arc<dyn AiClient>, /* ... */) {
//     let messages = vec![/* ... */];
//     let params = ChatParams { /* ... */ };
//     match client.chat_completion(messages, &params).await {
//         Ok(reply) => { /* ... */ }
//         Err(e) => { /* Handle error */ }
//     }
// }

// Declare and export the factory
pub mod factory;
pub use factory::create_ai_client;
