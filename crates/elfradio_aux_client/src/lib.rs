use elfradio_config::get_user_config_value;
use elfradio_config::ConfigError as ElfConfigError; // Import config error type
use reqwest::Client as ReqwestClient; // Assuming reqwest is used
use thiserror::Error;
use async_trait::async_trait; // For async trait methods
use tracing::{debug, error, info, warn, trace};
use serde::{Deserialize, Serialize}; // Added for request/response structs
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use std::sync::Arc; // 添加 Arc 导入

// Correct import for the trait
use elfradio_types::AuxServiceClient;

// Re-import the shared AiError type
use elfradio_types::AiError;

// Declare the Aliyun module
mod aliyun;
// Publicly export the Aliyun client
pub use aliyun::AliyunAuxClient;

// Define Request/Response Structs for Google Translate API v2
#[derive(Serialize, Debug)] // Added Debug
struct TranslateV2Request<'a> {
    q: &'a str,
    target: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a str>,
    format: &'static str, // "text"
    key: &'a str, // API Key
}

#[derive(Deserialize, Debug)]
struct TranslateV2ResponseData {
    translations: Vec<TranslateV2Translation>,
}

#[derive(Deserialize, Debug)]
struct TranslateV2Translation {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

#[derive(Deserialize, Debug)]
struct TranslateV2Response {
    // Handle potential errors returned in the response body if needed
    // error: Option<ApiErrorObject>,
    data: Option<TranslateV2ResponseData>, // Make data optional
}

// --- TTS V1 Structs ---
#[derive(Serialize, Debug)] // Added Debug
struct TtsSynthesisInput<'a> {
    text: &'a str,
}

#[derive(Serialize, Debug)] // Added Debug
struct TtsVoiceSelectionParams<'a> {
    #[serde(rename = "languageCode")]
    language_code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")] // Add skip_serializing_if
    name: Option<&'a str>,
}

#[derive(Serialize, Debug)] // Added Debug
struct TtsAudioConfig {
    #[serde(rename = "audioEncoding")]
    audio_encoding: String,
    #[serde(rename = "sampleRateHertz")]
    #[serde(skip_serializing_if = "Option::is_none")]
    sample_rate_hertz: Option<i32>,
    #[serde(rename = "speakingRate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    speaking_rate: Option<f64>,
    #[serde(rename = "volumeGainDb")]
    #[serde(skip_serializing_if = "Option::is_none")]
    volume_gain_db: Option<f64>,
}

#[derive(Serialize, Debug)] // Added Debug
struct SynthesizeSpeechRequest<'a> {
    input: TtsSynthesisInput<'a>,
    voice: TtsVoiceSelectionParams<'a>,
    #[serde(rename = "audioConfig")]
    audio_config: TtsAudioConfig,
}

#[derive(Deserialize, Debug)]
struct SynthesizeSpeechResponse {
    #[serde(rename = "audioContent")]
    audio_content: Option<String>, // Base64 encoded audio string
}

// --- STT V1 Structs ---
#[derive(Serialize, Debug)] // Added Debug
struct SttRecognitionAudio<'a> {
    content: &'a str, // Base64 encoded audio data
}

#[derive(Serialize, Debug)] // Added Debug
struct SttRecognitionConfig {
    encoding: String, // e.g., "LINEAR16"
    #[serde(rename = "sampleRateHertz")]
    sample_rate_hertz: i32, // Use i32 as per v1 API spec
    #[serde(rename = "languageCode")]
    language_code: String, // e.g., "en-US"
}

#[derive(Serialize, Debug)] // Added Debug
struct RecognizeRequest<'a> {
    config: SttRecognitionConfig,
    audio: SttRecognitionAudio<'a>,
}

#[derive(Deserialize, Debug)]
struct SpeechRecognitionAlternative {
    transcript: Option<String>, // Make optional for safety
}

#[derive(Deserialize, Debug)]
struct SpeechRecognitionResult {
    alternatives: Vec<SpeechRecognitionAlternative>,
}

#[derive(Deserialize, Debug)]
struct RecognizeResponse {
    results: Option<Vec<SpeechRecognitionResult>>, // Make optional for safety
}

/// Client for Google Auxiliary AI services (Translate, TTS, STT)
#[derive(Debug)]
pub struct GoogleAuxClient {
    api_key: String,
    http_client: ReqwestClient,
}

impl GoogleAuxClient {
    /// Creates a new GoogleAuxClient.
    /// Reads the API key directly from the user configuration file.
    pub fn new() -> Result<Self, AiError> {
        debug!("Attempting to create GoogleAuxClient, fetching API key from config...");

        // Fetch the API key using the config helper function
        // Assuming the key structure is ai_settings.google.api_key as per standard Config
        let key_result = get_user_config_value::<String>("ai_settings.google.api_key");

        match key_result {
            Ok(Some(key)) if !key.is_empty() => {
                info!("Successfully retrieved Google auxiliary API Key (from ai_settings.google.api_key) for GoogleAuxClient.");
                let http_client = ReqwestClient::new(); // Create HTTP client
                Ok(Self {
                    api_key: key,
                    http_client,
                })
            }
            Ok(Some(_)) => { // Key is empty
                warn!("Found Google auxiliary API Key (ai_settings.google.api_key) in user config, but it is empty.");
                Err(AiError::AuthenticationError(
                    "Google API Key (ai_settings.google.api_key) found but is empty in user configuration.".to_string(),
                ))
            }
            Ok(None) => { // Key not found in user config
                warn!("Google auxiliary API Key (ai_settings.google.api_key) not found in user configuration.");
                 Err(AiError::AuthenticationError(
                    "Google API Key (ai_settings.google.api_key) not found in user configuration.".to_string(),
                ))
            }
            Err(config_err) => { // Error reading/parsing config file
                error!("Failed to read configuration for Google auxiliary API Key (ai_settings.google.api_key): {}", config_err);
                // Map the ElfConfigError to AiError::ClientError (assuming no dedicated Config variant in elfradio_types::AiError)
                // Or map it to a specific ConfigError variant if it exists
                Err(AiError::ClientError(format!("Configuration error reading API key: {}", config_err)))
            }
        }
    }
}

#[async_trait]
impl AuxServiceClient for GoogleAuxClient {
    // Implemented translate method
    async fn translate(&self, text: &str, target_language: &str, source_language: Option<&str>) -> Result<String, AiError> {
        // API Key is already stored in self.api_key from the new() constructor
        let api_key = &self.api_key;

        let request_payload = TranslateV2Request {
            q: text,
            target: target_language,
            source: source_language,
            format: "text",
            key: api_key, // Pass the API key in the request body for v2
        };

        // Use the v2 endpoint known to work well with API keys
        let url = "https://translation.googleapis.com/language/translate/v2";

        debug!("Sending translation request to Google API v2...");
        trace!("Translate Request Payload: {:?}", request_payload); // Log payload at trace level

        let response = self.http_client // Use the client stored in self
            .post(url)
            .json(&request_payload)
            .send()
            .await
            .map_err(|e| {
                error!("Network error during Google Translate request: {}", e);
                // Use RequestError variant from elfradio_types::AiError
                AiError::RequestError(e.to_string()) // Changed Network to RequestError
            })?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
             error!("Failed to read Google Translate response body: {}", e);
             // Use ResponseParseError variant
             AiError::ResponseParseError(format!("Failed to read response body: {}", e))
        })?; // Read body once

        trace!("Translate Response Status: {}, Body: {}", status, response_text);

        if !status.is_success() {
            warn!("Google Translate API error. Status: {}, Body: {}", status, response_text);
            // Use ApiError variant from elfradio_types::AiError
            return Err(AiError::ApiError { // Use struct variant
                status: status.as_u16(),
                message: format!(
                    "Google Translate API request failed: {}",
                    response_text // Include response body in error
                ),
            });
        }

        // Parse the successful response
        let response_body: TranslateV2Response = serde_json::from_str(&response_text)
             .map_err(|e| {
                 error!("Failed to parse successful Google Translate response: {}", e);
                 // Use ResponseParseError variant
                 AiError::ResponseParseError(format!("Failed to parse translate response: {}", e))
             })?;

        // Extract the translation
        if let Some(data) = response_body.data {
             if let Some(translation) = data.translations.into_iter().next() {
                 debug!("Translation successful.");
                 Ok(translation.translated_text)
             } else {
                 // Use ResponseParseError variant
                 Err(AiError::ResponseParseError("No translation found in successful API response.".to_string()))
             }
        } else {
             // Use ResponseParseError variant
             Err(AiError::ResponseParseError("API response missing 'data' field.".to_string()))
        }
    }

    // Implemented text_to_speech method
    async fn text_to_speech(&self, text: &str, language_code: &str, voice_name: Option<&str>) -> Result<Vec<u8>, AiError> {
        let api_key = &self.api_key;

        let input = TtsSynthesisInput { text };
        let voice = TtsVoiceSelectionParams { language_code, name: voice_name };
        // Request LINEAR16 for easier processing later
        let audio_config = TtsAudioConfig {
            audio_encoding: "LINEAR16".to_string(),
            sample_rate_hertz: None, // Let API default based on voice/encoding
            speaking_rate: None, // Add if needed from config/params
            volume_gain_db: None, // Add if needed from config/params
        };

        let request_payload = SynthesizeSpeechRequest { input, voice, audio_config };

        let url = "https://texttospeech.googleapis.com/v1/text:synthesize";

        debug!("Sending TTS request to Google API v1...");
        trace!("TTS Request Payload: {:?}", request_payload); // Be careful logging sensitive text

        let response = self.http_client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            // Add the API key as a query parameter
            .query(&[("key", &self.api_key)])
            .json(&request_payload)
            .send()
            .await
            .map_err(|e| {
                error!("Network error during Google TTS request: {}", e);
                // Use RequestError variant from elfradio_types::AiError
                AiError::RequestError(e.to_string()) // Changed NetworkError to RequestError
            })?;

        let status = response.status();
        // Read the body as text first for potential error messages and trace logging
        let response_text = response.text().await.map_err(|e| {
             error!("Failed to read Google TTS response body: {}", e);
             // Use ResponseParseError variant
             AiError::ResponseParseError(format!("Failed to read response body: {}", e))
        })?;

        trace!("TTS Response Status: {}, Body: {}", status, response_text);

        if !status.is_success() {
            warn!("Google TTS API error. Status: {}, Body: {}", status, response_text);
            // Use ApiError variant
            return Err(AiError::ApiError {
                status: status.as_u16(),
                message: format!(
                    "Google TTS API request failed: {}",
                    response_text
                ),
            });
        }

        // Parse the successful response text
        let response_body: SynthesizeSpeechResponse = serde_json::from_str(&response_text)
             .map_err(|e| {
                 error!("Failed to parse successful Google TTS response: {}", e);
                 // Use ResponseParseError variant
                 AiError::ResponseParseError(format!("Failed to parse TTS response: {}", e))
             })?;

        // Decode the Base64 audio content
        if let Some(audio_base64) = response_body.audio_content {
            BASE64_STANDARD.decode(audio_base64).map_err(|e| {
                error!("Failed to decode Base64 audio content from Google TTS: {}", e);
                // Use ResponseParseError variant
                AiError::ResponseParseError(format!("Failed to decode Base64 audio: {}", e))
            })
        } else {
            // Use ResponseParseError variant
            Err(AiError::ResponseParseError("Google TTS response missing 'audioContent' field.".to_string()))
        }
    }

    // Implemented speech_to_text method
    async fn speech_to_text(&self, audio_data: &[u8], sample_rate_hertz: u32, language_code: &str) -> Result<String, AiError> {
        let api_key = &self.api_key;

        // 1. Base64 encode the audio data
        let audio_base64 = BASE64_STANDARD.encode(audio_data);

        // 2. Prepare the request payload
        // Assuming LINEAR16 encoding based on typical input
        let config = SttRecognitionConfig {
            encoding: "LINEAR16".to_string(),
            sample_rate_hertz: sample_rate_hertz as i32, // Cast to i32 for API
            language_code: language_code.to_string(),
        };
        let audio = SttRecognitionAudio { content: &audio_base64 };
        let request_payload = RecognizeRequest { config, audio };

        // 3. Define URL and Headers
        let url = "https://speech.googleapis.com/v1/speech:recognize";

        debug!("Sending STT request to Google API...");
        // Avoid tracing the entire audio data, log its length instead
        trace!("STT Request Payload: config={:?}, audio_len={}", request_payload.config, request_payload.audio.content.len());

        let response = self.http_client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            // Add the API key as a query parameter
            .query(&[("key", &self.api_key)])
            .json(&request_payload)
            .send()
            .await
            .map_err(|e| {
                error!("Network error during Google STT request: {}", e);
                // Use RequestError variant from elfradio_types::AiError
                AiError::RequestError(e.to_string()) // Changed NetworkError to RequestError
            })?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
             error!("Failed to read Google STT response body: {}", e);
             // Use ResponseParseError variant
             AiError::ResponseParseError(format!("Failed to read response body: {}", e))
        })?;

        trace!("STT Response Status: {}, Body: {}", status, response_text);

        if !status.is_success() {
            warn!("Google STT API error. Status: {}, Body: {}", status, response_text);
            // Use ApiError variant
            return Err(AiError::ApiError {
                status: status.as_u16(),
                message: format!(
                    "Google STT API request failed: {}",
                    response_text
                ),
            });
        }

        // 5. Parse the successful response
        let response_body: RecognizeResponse = serde_json::from_str(&response_text)
             .map_err(|e| {
                 error!("Failed to parse successful Google STT response: {}", e);
                 // Use ResponseParseError variant
                 AiError::ResponseParseError(format!("Failed to parse STT response: {}", e))
             })?;

        // 6. Extract the transcript
        // Get the first transcript from the first result's first alternative
        let transcript = response_body.results
            .and_then(|res| res.into_iter().next()) // Get first result
            .and_then(|r| r.alternatives.into_iter().next()) // Get first alternative
            .and_then(|alt| alt.transcript) // Get the transcript Option<String>
            .unwrap_or_default(); // Default to empty string if no transcript found

        if transcript.is_empty() {
            warn!("Google STT returned a successful response but no transcript was found.");
            // Depending on requirements, you might want to return an error here instead.
            // For now, returning an empty string is acceptable.
        } else {
            debug!("STT successful. Transcript length: {}", transcript.len());
        }

        Ok(transcript)
    }
}
