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
            // 1. Get Google specific config
            let google_config = config.ai_settings.google.as_ref().ok_or_else(|| {
                error!("GoogleGemini provider selected but [ai_settings.google] configuration is missing.");
                    AiError::ClientError("Google API Key configuration is missing for GoogleGemini provider.".to_string())
                })?;

            // 2. Extract the Google API Key
            let google_api_key = google_config.api_key.as_ref().ok_or_else(|| {
                error!("Google API Key is missing in [ai_settings.google].");
                AiError::AuthenticationError("Google API Key not found or empty in user config.".to_string())
            })?;

            // 3. Define the Google OpenAI-compatible endpoint
            // --- Ensure the endpoint includes /v1beta/openai or similar, not just generativelanguage.googleapis.com ---
            let google_compat_endpoint = "https://generativelanguage.googleapis.com/v1beta/openai";


            // 4. Manually create an OpenAICompatibleConfig for this specific case
            let compat_config_for_google = OpenAICompatibleConfig {
                name: Some("Google Gemini (via OpenAI Compat)".to_string()), // Optional name
                base_url: Some(google_compat_endpoint.to_string()),
                api_key: Some(google_api_key.clone()), // Use the Google Key
                preferred_model: google_config.preferred_model.clone(), // Use preferred model from google config
            };

            // 5. Create the OpenAICompatibleClient using the constructed config
            match OpenAICompatibleClient::new(compat_config_for_google) {
                Ok(client) => Ok(Arc::new(client)),
                Err(e) => {
                    // This error should ideally not happen now if key/URL are correct,
                    // but handle potential client creation errors.
                    error!("Failed to create OpenAICompatibleClient for Google Gemini: {}", e);
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
            // Ensure this part correctly reads from config.ai_settings.openai_compatible
            let openai_config = config.ai_settings.openai_compatible.as_ref().ok_or_else(|| {
                 error!("OpenAICompatible provider selected but [ai_settings.openai_compatible] configuration is missing.");
                AiError::ClientError("OpenAICompatible configuration is missing.".to_string())
             })?;
             // Ensure OpenAICompatibleClient::new takes OpenAICompatibleConfig
             match OpenAICompatibleClient::new(openai_config.clone()) { // Pass by value (clone)
                 Ok(client) => Ok(Arc::new(client)),
                 Err(e) => Err(e), // Propagate error
             }
        }
        // Note: No fallback or default provider creation logic is needed here,
        // as the initial check handles the None case.
    }
} 