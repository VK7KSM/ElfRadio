//! Implementation of the AiClient trait for OpenAI-compatible APIs using the async-openai crate.

use crate::{AiClient, ChatMessage, ChatParams, SttParams, TtsParams}; // Added SttParams
use async_openai::{
    config::OpenAIConfig, // Use the specific config type
    error::OpenAIError,
    types::{
        // Import necessary builders and types for message construction
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs, // Removed Role import
    },
    Client as OpenAIClientSdk, // Rename SDK client for clarity
};
use async_trait::async_trait;
use elfradio_types::OpenAICompatibleConfig; // Correct path assumed
use std::sync::Arc;
use tracing::{debug, error, info, warn}; // Add necessary tracing imports
use elfradio_types::AiError;
use elfradio_config::get_user_config_value;

/// AI Client implementation for interacting with OpenAI-compatible APIs
/// (e.g., OpenAI itself, StepFun Chat, DeepSeek API).
#[derive(Debug, Clone)]
pub struct OpenAICompatibleClient {
    /// The underlying client from the async-openai SDK.
    client: OpenAIClientSdk<OpenAIConfig>,
    /// Shared configuration specific to this OpenAI-compatible provider.
    config: Arc<OpenAICompatibleConfig>,
}

/// Helper function to map OpenAIError to AiError
fn map_openai_error(err: OpenAIError) -> AiError {
    tracing::warn!("Mapping OpenAI Error: {:?}", err); // Add tracing
    match err {
        OpenAIError::ApiError(api_err) => {
            // 修复：尝试将 code 解析为 u16，如果无法解析则使用默认值 0
            let status_code: u16 = api_err.code.as_ref()
                .and_then(|s| s.parse::<u16>().ok()) // 尝试将字符串解析为 u16
                .unwrap_or(0); // 如果 code 为 None 或解析失败，则默认为 0
            
            AiError::ApiError { // 使用 elfradio_types::AiError 中的结构体变体
                status: status_code,
                message: format!("OpenAI API error: Type={:?}, Code={:?}, Message={}, Param={:?}",
                    api_err.r#type, api_err.code, api_err.message, api_err.param),
            }
        }
        // Note: OpenAIError does not seem to have a distinct RateLimit variant in async-openai 0.20.
        // Rate limit errors typically come as ApiError with status 429.
        OpenAIError::Reqwest(e) => AiError::RequestError(format!("HTTP request failed: {}", e)), // Changed Network to RequestError
        // Map other specific OpenAIError variants if necessary
        OpenAIError::StreamError(s) => AiError::ResponseParseError(format!("Stream error: {}", s)), // Map to ResponseParseError
        OpenAIError::FileSaveError(s) | OpenAIError::FileReadError(s) => AiError::ClientError(format!("File IO error: {}", s)), // Map File IO to ClientError
        OpenAIError::InvalidArgument(s) => AiError::InvalidInput(format!("Invalid argument: {}", s)), // Map to InvalidInput
        _ => AiError::ClientError(format!("Unhandled OpenAI client error: {}", err)), // Fallback to ClientError
    }
}

impl OpenAICompatibleClient {
    /// Creates a new OpenAI-compatible client instance.
    ///
    /// # Arguments
    /// * `config` - Configuration specific to the OpenAI-compatible provider.
    ///
    /// # Returns
    /// A `Result` containing the new client instance or an `AiError::ClientError`
    /// if essential configuration (base URL, API key) is missing.
    pub fn new(config: OpenAICompatibleConfig) -> Result<Self, AiError> {
        debug!(
            "Initializing OpenAICompatibleClient for name: {:?}, base_url: {:?}",
            config.name, config.base_url
        );

        let api_base = config.base_url.clone().ok_or_else(|| {
            error!("Missing required configuration: base_url for OpenAICompatibleClient");
            AiError::ClientError("Missing required configuration: base_url".to_string())
        })?;

        let api_key_result = get_user_config_value::<String>("ai_settings.openai_compatible.api_key")
            .map_err(|e| {
                error!("Failed to read OpenAI-compatible API key from user config: {}", e);
                AiError::ClientError(format!("Failed to read API key from user config: {}", e))
            })?;
        
        let api_key = match api_key_result {
            Some(key) if !key.is_empty() => key,
            _ => {
                error!("OpenAI Compatible API Key not found or empty in user config.");
                return Err(AiError::AuthenticationError(
                    "OpenAI Compatible API Key not found or empty in user config.".to_string()
                ));
            }
        };

        // Create the SDK configuration, setting the crucial base URL and API key
        let sdk_config = OpenAIConfig::new()
            .with_api_base(api_base)
            .with_api_key(api_key);
        // Add other config options like org_id if needed from config

        // Create the SDK client instance
        let client = OpenAIClientSdk::with_config(sdk_config);

        info!(
            "OpenAICompatibleClient initialized successfully for name: {:?}",
            config.name
        );
        Ok(Self {
            client,
            config: Arc::new(config), // Store the original config in an Arc
        })
    }
}

#[async_trait]
impl AiClient for OpenAICompatibleClient {
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        params: &ChatParams,
    ) -> Result<String, AiError> {
        // *** Fix Start: Correctly map ChatMessage to enum variants using builders ***
        let request_messages: Result<Vec<ChatCompletionRequestMessage>, AiError> = messages
            .into_iter()
            .map(|msg| -> Result<ChatCompletionRequestMessage, AiError> {
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
                            .content(msg.content)
                            .build()
                            .map_err(|e| AiError::ClientError(format!("Failed to build assistant message: {}", e)))?
                    )),
                    unknown_role => {
                         warn!("Unknown role '{}' encountered, treating as user.", unknown_role);
                         // Defaulting unknown roles to User
                         Ok(ChatCompletionRequestMessage::User(
                             ChatCompletionRequestUserMessageArgs::default()
                                 .content(msg.content)
                                 .build()
                                 .map_err(|e| AiError::ClientError(format!("Failed to build default user message: {}", e)))?
                         ))
                    }
                }
            })
            .collect(); // Collect into a Result<Vec<_>, AiError>

        let request_messages = request_messages?; // Propagate error if any message failed to build
        // *** Fix End ***

        // 2. Determine the model to use: params -> config -> error
        let model = params
            .model // Now Option<String>
            .as_deref() // Get &str if Some, or None
            .or(self.config.preferred_model.as_deref()) // Fallback to config preferred_model if params.model is None
            .ok_or_else(|| {
                error!("No model specified in ChatParams or OpenAICompatibleConfig.");
                AiError::ClientError("No model specified in params or provider config".to_string())
            })? // Return error if neither is set
            .to_string(); // Convert the final &str back to String
        debug!("Using model for chat completion: {}", model);

        // 3. Build the request arguments
        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder.model(&model).messages(request_messages);

        // Apply optional parameters from ChatParams
        if let Some(temp) = params.temperature {
            request_builder.temperature(temp);
        }
        if let Some(p) = params.top_p {
            request_builder.top_p(p);
        }
        if let Some(tokens) = params.max_tokens {
            // async-openai expects u16 for max_tokens
            if tokens <= u16::MAX as u32 {
                 request_builder.max_tokens(tokens as u16);
            } else {
                 warn!("max_tokens ({}) exceeds u16::MAX, clamping.", tokens);
                 request_builder.max_tokens(u16::MAX);
            }
        }
        // Add other parameters like stream: false if needed

        let request = request_builder
            .build()
            .map_err(|e| AiError::ClientError(format!("Failed to build request: {}", e)))?;

        // 4. Make the API call
        debug!("Sending chat completion request to API base...");
        let response = self.client.chat().create(request).await.map_err(map_openai_error)?; // Apply map_err

        // 5. Process the response
        let choice = response.choices.into_iter().next().ok_or_else(|| {
            error!("API response contained no choices.");
            AiError::ResponseParseError("API response contained no choices".to_string())
        })?;

        // Extract content, handling potential None case (though usually present for non-streaming)
        choice.message.content.ok_or_else(|| {
            error!("API response choice contained no content.");
            AiError::ResponseParseError("API response choice contained no content".to_string())
        })
    }

    async fn text_to_speech(
        &self,
        _text: &str,
        _params: &TtsParams,
    ) -> Result<Vec<u8>, AiError> {
        warn!(
            "TTS requested for OpenAICompatibleClient (Name: {:?}), which is not supported.",
            self.config.name
        );
        Err(AiError::NotSupported(
            "TTS is not supported by this generic OpenAI-compatible client implementation."
                .to_string(),
        ))
    }

    async fn list_models(&self) -> Result<Vec<String>, AiError> {
        debug!("Requesting model list from API base...");
        let response = self.client.models().list().await.map_err(map_openai_error)?; // Apply map_err

        let model_ids: Vec<String> = response.data.into_iter().map(|model| model.id).collect();
        debug!("Received {} models.", model_ids.len());
        Ok(model_ids)
    }

    // Implement speech_to_text if needed, likely returning NotSupported by default as well
    async fn speech_to_text(
         &self,
        _audio_data: &[u8], // Use _ prefix for unused parameters, match trait &[u8]
        _params: &SttParams,
    ) -> Result<String, AiError> {
        warn!("speech_to_text called on OpenAICompatibleClient, but it's not implemented yet.");
         Err(AiError::NotSupported(
            "speech_to_text not implemented for OpenAICompatibleClient".to_string()
         ))
    }
} 