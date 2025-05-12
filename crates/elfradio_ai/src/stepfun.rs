//! Implementation of the AiClient trait for StepFun Text-to-Speech API.

use async_trait::async_trait;
use reqwest::Client;
use reqwest::header;
use serde::Serialize;
use tracing::{debug, error, warn, instrument};
use elfradio_types::{ChatMessage, ChatParams, AiError, StepFunTtsConfig};
use crate::{AiClient, SttParams, TtsParams};
use std::sync::Arc;
use elfradio_config::get_user_config_value;

#[allow(dead_code)]
const STEPFUN_TTS_API_URL: &str = "https://api.stepfun.com/v1/audio/speech";
// --- Voice ID ---
// Use 'wenrounvsheng' as the default for all languages as requested.
// const DEFAULT_VOICE: &str = "wenrounvsheng";
// --- API Parameters ---
#[allow(dead_code)]
const PARAM_SPEED_MIN: f32 = 0.5;
#[allow(dead_code)]
const PARAM_SPEED_MAX: f32 = 2.0;
#[allow(dead_code)]
const PARAM_VOLUME_MIN: f32 = 0.1;
#[allow(dead_code)]
const PARAM_VOLUME_MAX: f32 = 2.0;
// --- Model & Limits ---
#[allow(dead_code)]
const DEFAULT_MODEL_TTS: &str = "step-tts-mini";
const MAX_INPUT_CHARS: usize = 1000; // Character limit per request

/// Client for interacting with StepFun AI services (specifically TTS in this case).
#[derive(Debug)]
pub struct StepFunTtsClient {
    pub http_client: Client,
    pub config: Arc<StepFunTtsConfig>,
    api_key: String,
}

#[derive(Debug, Serialize)]
struct StepFunTtsRequest<'a> {
    model: &'static str,
    input: &'a str,
    voice: String,
    response_format: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    speed: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    volume: Option<f32>,
}

impl StepFunTtsClient {
    /// Creates a new StepFun TTS client.
    pub fn new(config: StepFunTtsConfig) -> Result<Self, AiError> {
        let api_key_result = get_user_config_value::<String>("ai_settings.stepfun_tts.api_key")
            .map_err(|e| {
                error!("Failed to read StepFun TTS API key from user config: {}", e);
                AiError::ClientError(format!("Failed to read API key from user config: {}", e))
            })?;
        
        let api_key = match api_key_result {
            Some(key) if !key.is_empty() => key,
            _ => {
                error!("StepFun TTS API Key not found or empty in user config.");
                return Err(AiError::AuthenticationError(
                    "StepFun TTS API Key not found or empty in user config.".to_string()
                ));
            }
        };

        Ok(Self {
            http_client: Client::new(),
            config: Arc::new(config),
            api_key,
        })
    }

    // --- New Method for Voice Cloning (Provider Specific) ---
    /// Creates a new voice clone based on an uploaded audio file.
    /// Returns the new voice_id on success.
    /// (Implementation deferred to later phase)
    pub async fn create_voice_clone(
        &self,
        _audio_file_id: &str, // ID obtained from uploading WAV/MP3 with purpose=storage
        _text_prompt: &str,  // Text corresponding to the audio content
        _sample_text: Option<&str>, // Optional text for generating a sample
    ) -> Result<String, AiError> {
        // TODO: Implement StepFun voice clone API call (/v1/audio/voices)
        // This likely involves another POST request with specific parameters
        // including audio_file_id, text_prompt, etc. The response should contain
        // the new voice_id.
        error!("StepFun voice cloning via /v1/audio/voices is not yet implemented.");
        Err(AiError::NotSupported("Voice cloning not implemented".to_string()))
    }
    // --- End New Method ---

    #[instrument(skip(self, text, params), fields(text_len = text.chars().count()))]
    pub async fn text_to_speech(&self, text: &str, params: &TtsParams) -> Result<Vec<u8>, AiError> {
        debug!("Starting StepFun TTS synthesis...");

        if text.chars().count() > MAX_INPUT_CHARS {
            warn!("Input text exceeds StepFun limit ({} > {}).", text.chars().count(), MAX_INPUT_CHARS);
            return Err(AiError::RequestError(format!(
                "Input text exceeds {} character limit",
                MAX_INPUT_CHARS
            )));
        }

        let selected_voice = if params.voice_id.is_empty() {
            "wenrounvsheng".to_string()
        } else {
            params.voice_id.clone()
        };

        let response_format_req = if params.output_format.is_empty() {
            "wav".to_string()
        } else {
            params.output_format.clone()
        };
        let response_format = match response_format_req.to_lowercase().as_str() {
            "wav" | "mp3" | "flac" | "opus" | "pcm" => response_format_req.to_lowercase(),
            _ => {
                warn!("Unsupported format '{}' requested, defaulting to wav for StepFun.", response_format_req);
                "wav".to_string()
            }
        };

        let speed = params.speed.map(|s| s.clamp(0.5, 2.0));
        let volume = params.volume.map(|v| v.clamp(0.1, 2.0));

        let request_payload = StepFunTtsRequest {
            model: "step-tts-mini",
            input: text,
            voice: selected_voice,
            response_format: &response_format,
            speed,
            volume,
        };

        debug!("Sending StepFun TTS request payload: {:?}", request_payload);

        let response = self.http_client
            .post("https://api.stepfun.com/v1/audio/speech")
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .json(&request_payload)
            .send()
            .await
            .map_err(|e| AiError::RequestError(format!("Network error calling StepFun TTS: {}", e)))?;

        if response.status().is_success() {
            let audio_bytes = response.bytes().await.map_err(|e| {
                AiError::ResponseParseError(format!("Failed to read StepFun audio bytes: {}", e))
            })?;
            debug!("StepFun TTS synthesis successful ({} bytes)", audio_bytes.len());
            Ok(audio_bytes.to_vec())
        } else {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("Failed to read error body: {}", e));
            error!("StepFun TTS API error: Status {}, Body: {}", status, error_body);
            Err(AiError::ApiError {
                status: status.as_u16(),
                message: format!("StepFun API Error: {}", error_body),
            })
        }
    }
}

#[async_trait]
impl AiClient for StepFunTtsClient {
    async fn chat_completion(
        &self,
        _messages: Vec<ChatMessage>,
        _params: &ChatParams,
    ) -> Result<String, AiError> {
        Err(AiError::NotSupported("Chat completion not supported by StepFunTtsClient".to_string()))
    }

    #[instrument(skip(self, _text, _params))]
    async fn text_to_speech(
        &self,
        _text: &str,
        _params: &TtsParams,
    ) -> Result<Vec<u8>, AiError> {
        warn!("text_to_speech called on StepFunTtsClient, but it's not implemented yet.");
        Err(AiError::NotSupported(
            "text_to_speech not yet implemented for StepFunTtsClient".to_string(),
        ))
    }

    #[instrument(skip(self, _audio_data, _params))]
    async fn speech_to_text(
        &self,
        _audio_data: &[u8],
        _params: &SttParams,
    ) -> Result<String, AiError> {
        warn!("speech_to_text called on StepFunTtsClient, but it's not implemented (primarily a TTS client).");
        Err(AiError::NotSupported(
            "speech_to_text not implemented for StepFunTtsClient".to_string()
        ))
    }

    async fn list_models(&self) -> Result<Vec<String>, AiError> {
        Err(AiError::NotSupported("Model listing not supported by StepFunTtsClient".to_string()))
    }
} 