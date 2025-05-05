//! Implementation of the AiClient trait for Google Gemini via its OpenAI-compatible layer.

use crate::{AiClient, ChatMessage, ChatParams, SttParams, TtsParams}; // Use local AiClient trait
use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client as OpenAIClientSdk, // Use the async-openai client
};
use async_trait::async_trait;
use elfradio_types::{GoogleConfig, AiError}; // Use AiError from types
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// AI Client implementation for interacting with Google Gemini via OpenAI-compatible API.
#[derive(Debug, Clone)]
pub struct GoogleAiClient {
    openai_client: OpenAIClientSdk<OpenAIConfig>, // Internally uses the async-openai client
    config: Arc<GoogleConfig>, // Store Google-specific config (like preferred_model)
}

// Helper function to map OpenAIError to AiError (can be reused or adapted)
fn map_openai_error_for_google(err: OpenAIError) -> AiError {
    warn!("Mapping OpenAI Error (from Google Compatible Layer): {:?}", err);
    match err {
        OpenAIError::ApiError(api_err) => {
            let status_code: u16 = api_err.code.as_ref()
                .and_then(|s| s.parse::<u16>().ok()) // Try parsing string code
                .unwrap_or(0); // Default to 0 if code is None or parsing fails
            AiError::ApiError { // Use struct variant
                status: status_code,
                message: format!("Google API (via OpenAI Compat) error: Type={:?}, Code={:?}, Message={} Param={:?}",
                                    api_err.r#type, api_err.code, api_err.message, api_err.param),
            }
        }
        OpenAIError::Reqwest(e) => AiError::RequestError(format!("HTTP request failed (Google via OpenAI Compat): {}", e)),
        // Add specific mapping for RateLimitError if needed and if AiError supports it
        _ => AiError::ClientError(format!("Unhandled OpenAI client error (Google via OpenAI Compat): {}", err)),
    }
}


impl GoogleAiClient {
    /// Creates a new Google AI client instance using the OpenAI-compatible layer.
    pub fn new(config: GoogleConfig) -> Result<Self, AiError> {
        info!("Initializing GoogleAiClient (using OpenAI compatible layer)...");

        let api_key = config.api_key.clone().ok_or_else(|| {
            error!("Missing required configuration: google.api_key");
            AiError::AuthenticationError("Missing required Google API Key.".to_string())
        })?;

        // Google's OpenAI-compatible endpoint
        let api_base = "https://generativelanguage.googleapis.com/v1beta/openai".to_string();

        let sdk_config = OpenAIConfig::new()
            .with_api_base(api_base)
            .with_api_key(api_key);

        let openai_client = OpenAIClientSdk::with_config(sdk_config);

        info!("GoogleAiClient (OpenAI Compat Layer) initialized successfully.");
        Ok(Self {
            openai_client,
            config: Arc::new(config),
        })
    }
}

#[async_trait]
impl AiClient for GoogleAiClient {
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        params: &ChatParams,
    ) -> Result<String, AiError> {
        debug!("Starting Google Gemini chat completion (via OpenAI Compat Layer)...");

        let request_messages: Result<Vec<ChatCompletionRequestMessage>, AiError> = messages
            .into_iter()
            .map(|msg| {
                match msg.role.to_lowercase().as_str() {
                    "system" => Ok(ChatCompletionRequestMessage::System(
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(msg.content)
                            .build()
                            .map_err(|e| AiError::ClientError(format!("Failed to build system message: {}", e)))?
                    )),
                    "user" => Ok(ChatCompletionRequestMessage::User(
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(msg.content)
                            .build()
                            .map_err(|e| AiError::ClientError(format!("Failed to build user message: {}", e)))?
                    )),
                    "assistant" => Ok(ChatCompletionRequestMessage::Assistant(
                        ChatCompletionRequestAssistantMessageArgs::default()
                            .content(msg.content) // 直接传递 String
                            .build()
                            .map_err(|e| AiError::ClientError(format!("Failed to build assistant message: {}", e)))?
                    )),
                    _ => {
                        warn!("Unknown role '{}', treating as 'user'.", msg.role);
                        Ok(ChatCompletionRequestMessage::User(
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(msg.content)
                                .build()
                                .map_err(|e| AiError::ClientError(format!("Failed to build default user message: {}", e)))?
                        ))
                    }
                }
            })
            .collect(); // Collect into Result<Vec<_>, AiError>

        let request_messages = request_messages?; // Propagate error if building failed

        // Determine model name: Use specific model from params, or preferred from config, or default
        let model_name = params.model.as_deref()
            .or(self.config.preferred_model.as_deref())
            .unwrap_or("gemini-2.0-flash"); // Use the correct default string

        debug!("Using Gemini model (via OpenAI Compat): {}", model_name);

        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder.model(model_name).messages(request_messages); // Pass model string

        if let Some(temp) = params.temperature { request_builder.temperature(temp); }
        if let Some(p) = params.top_p { request_builder.top_p(p); }
        if let Some(tokens) = params.max_tokens {
             if tokens <= u16::MAX as u32 { request_builder.max_tokens(tokens as u16); }
             else { warn!("max_tokens ({}) exceeds u16::MAX, clamping.", tokens); request_builder.max_tokens(u16::MAX); }
        }
        // Add timeout if async-openai supports it in builder or client config

        let request = request_builder.build()
            .map_err(|e| AiError::ClientError(format!("Failed to build OpenAI Compat request: {}", e)))?;

        debug!("Sending chat completion request via OpenAI Compat Layer...");
        let response = self.openai_client.chat().create(request).await
            .map_err(map_openai_error_for_google)?; // Use explicit mapping

        // Extract response text
        let choice = response.choices.into_iter().next().ok_or_else(|| {
            AiError::ResponseParseError("Google API (via OpenAI Compat) response contained no choices".to_string())
        })?;
        choice.message.content.ok_or_else(|| {
            AiError::ResponseParseError("Google API (via OpenAI Compat) response choice contained no content".to_string())
        })
    }

    // --- TTS/STT/Translate are NOT handled by this client ---
    async fn text_to_speech( &self, _text: &str, _params: &TtsParams ) -> Result<Vec<u8>, AiError> {
        Err(AiError::NotSupported("TTS is handled by AuxServiceClient".to_string()))
    }

    async fn speech_to_text( &self, _audio_data: &[u8], _params: &SttParams ) -> Result<String, AiError> {
        Err(AiError::NotSupported("STT is handled by AuxServiceClient".to_string()))
    }

    async fn list_models(&self) -> Result<Vec<String>, AiError> {
        // Use the underlying async-openai client's list_models if needed,
        // but it will list OpenAI models unless pointed at Google's specific endpoint for models.
        // Google's compatible layer might not support listing models this way.
        warn!("list_models called on GoogleAiClient (OpenAI Compat), may not list Google models accurately.");
        // Ok(vec!["gemini-2.0-flash".to_string()]) // Return known model
         Err(AiError::NotSupported("Listing Google models via OpenAI compat layer not reliably supported.".to_string()))
    }
} 