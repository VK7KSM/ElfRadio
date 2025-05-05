//! Factory function to create AI client instances based on configuration.

use std::sync::Arc;
use tracing::{info, error};

// Import necessary types from elfradio_types
// Ensure AiError, Config, OpenAICompatibleConfig are correctly imported
use elfradio_types::{AiProvider, Config, OpenAICompatibleConfig, AiError};

// Import client implementations and the core trait/error from this crate
use super::{AiClient, GoogleAiClient, StepFunTtsClient, OpenAICompatibleClient}; // 恢复 GoogleAiClient 导入

/// Creates an AI client based on the AI configuration found within the main Config.
/// Returns `AiError::ProviderNotSpecified` if no provider is set in the config.
/// Returns `AiError::ClientError` if the required configuration for the specified provider is missing.
pub async fn create_ai_client(config: &Config) -> Result<Arc<dyn AiClient + Send + Sync>, AiError> {
    // Reference the ai_settings directly from the main config
    let ai_config = &config.ai_settings;

    // Check if a provider is specified at the beginning.
    let provider = match ai_config.provider.as_ref() {
        Some(p) => p, // Provider is specified, proceed.
        None => {
            info!("AI provider is not specified in configuration. Cannot create AI client.");
            // Return the specific error if no provider is configured.
            return Err(AiError::ProviderNotSpecified);
        }
    };

    info!("Attempting to create AI client for provider: {:?}", provider);

    // Proceed with creating the client based on the specified provider.
    match provider {
        AiProvider::GoogleGemini => {
            info!("Configuring OpenAICompatibleClient for Google Gemini provider...");
            // Extract Google API Key from ai_config.google
            let google_api_key = config.ai_settings.google.as_ref()
                .and_then(|g_cfg| g_cfg.api_key.as_ref())
                .ok_or_else(|| {
                    error!("GoogleGemini provider selected but google.api_key is missing in ai_settings.");
                    AiError::ClientError("Google API Key configuration is missing for GoogleGemini provider.".to_string())
                })?;

            // Define the Google Gemini OpenAI-compatible base URL - CORRECTED
            let base_url = "https://generativelanguage.googleapis.com/v1beta/openai/"; // Correct URL with /openai/

            // Create a temporary OpenAICompatibleConfig specifically for Gemini - CORRECTED
            let gemini_compat_config = OpenAICompatibleConfig {
                name: Some("Google Gemini (via OpenAI Compat Layer)".to_string()), // Informative name
                base_url: Some(base_url.to_string()),
                api_key: Some(google_api_key.clone()), // Use the Google API Key
                preferred_model: config.ai_settings.google.as_ref().and_then(|g| g.preferred_model.clone()), // Use preferred model from google config if set
                // DELETE: tts_enabled: false, // Removed non-existent field
                // DELETE: stt_enabled: false, // Removed non-existent field
            };

            // Create the OpenAICompatibleClient instance using this temporary config
            match OpenAICompatibleClient::new(gemini_compat_config) {
                Ok(client) => Ok(Arc::new(client) as Arc<dyn AiClient + Send + Sync>),
                Err(e) => {
                    error!("Failed to create OpenAICompatibleClient for Google Gemini: {}", e);
                    // Propagate the error from OpenAICompatibleClient::new
                    Err(e)
                }
            }
        }
        AiProvider::StepFunTTS => {
            let stepfun_config = ai_config.stepfun_tts.as_ref().ok_or_else(|| {
                error!("StepFunTTS provider selected but configuration is missing.");
                AiError::ClientError("StepFunTTS configuration is missing.".to_string())
            })?;
            // Ensure StepFunTtsClient::new aligns with expected signature
            let client = StepFunTtsClient::new(stepfun_config.clone())?; // Assuming new returns Result<Self, AiError>
            Ok(Arc::new(client) as Arc<dyn AiClient + Send + Sync>)
        }
        AiProvider::OpenAICompatible => {
            let openai_config = ai_config.openai_compatible.as_ref().ok_or_else(|| {
                error!("OpenAICompatible provider selected but configuration is missing.");
                AiError::ClientError("OpenAICompatible configuration is missing.".to_string())
             })?;
            // Ensure OpenAICompatibleClient::new aligns with expected signature
            let client = OpenAICompatibleClient::new(openai_config.clone())?; // Assuming new returns Result<Self, AiError>
            Ok(Arc::new(client) as Arc<dyn AiClient + Send + Sync>)
        }
        // Note: No fallback or default provider creation logic is needed here,
        // as the initial check handles the None case.
    }
} 