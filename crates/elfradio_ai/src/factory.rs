//! Factory function to create AI client instances based on configuration.

use std::sync::Arc;
use tracing::{info, error};

// MODIFIED: Ensure all necessary types from elfradio_types are imported.
// ChatMessage and ChatParams are not directly used in this file's function signatures,
// but other types like AiProvider, Config, specific configs (OpenAICompatibleConfig, GoogleConfig, StepFunTtsConfig),
// and AiError are crucial.
use elfradio_types::{
    AiProvider, Config, OpenAICompatibleConfig, AiError,
    // AuxServiceClient and AuxServiceProvider might be needed if creating aux clients here too
};

// Import client implementations and the core AiClient trait from this crate (super)
use super::{AiClient, StepFunTtsClient, OpenAICompatibleClient};
// If Aux clients were created here, you might have:
// use elfradio_aux_client::GoogleAuxClient;

/// Creates an AI client based on the AI configuration found within the main Config.
/// Returns `AiError::ProviderNotSpecified` if no provider is set in the config.
/// Returns `AiError::ClientError` if the required configuration for the specified provider is missing.
pub async fn create_ai_client(config: &Config) -> Result<Arc<dyn AiClient + Send + Sync>, AiError> {
    // Reference the ai_settings directly from the main config
    let ai_config = &config.ai_settings; // AiConfig is part of elfradio_types::Config

    // Check if a provider is specified at the beginning.
    let provider = match ai_config.provider.as_ref() { // ai_config.provider is Option<AiProvider> from elfradio_types
        Some(p) => p, // Provider is specified, proceed.
        None => {
            info!("Primary AI (LLM) provider is not specified in `ai_settings.provider`. ElfRadio AI client cannot be created. LLM features will be unavailable until configured.");
            // Return the specific error if no provider is configured.
            return Err(AiError::ProviderNotSpecified); // AiError from elfradio_types
        }
    };

    info!("Attempting to create AI client for provider: {:?}", provider);

    // Proceed with creating the client based on the specified provider.
    match provider {
        AiProvider::GoogleGemini => {
            info!("Attempting to initialize AiClient for GoogleGemini provider...");
            // 1. Get Google specific config
            let google_config = config.ai_settings.google.as_ref().ok_or_else(|| {
                error!("AiClient (GoogleGemini): Initialization failed. The '[ai_settings.google]' configuration block is missing in elfradio_config.toml or default config.");
                    AiError::ClientError("Google API Key configuration is missing for GoogleGemini provider.".to_string())
                })?;

            // 2. Extract the Google API Key
            let google_api_key = google_config.api_key.as_ref().ok_or_else(|| {
                error!("AiClient (GoogleGemini): Initialization failed. The 'api_key' is missing or empty within the '[ai_settings.google]' configuration block.");
                AiError::AuthenticationError("Google API Key not found or empty in user config.".to_string())
            })?;

            // 3. Define the Google OpenAI-compatible endpoint
            let google_compat_endpoint = "https://generativelanguage.googleapis.com/v1beta/openai";


            // 4. Manually create an OpenAICompatibleConfig for this specific case
            let compat_config_for_google = OpenAICompatibleConfig {
                name: Some("Google Gemini (via OpenAI Compat)".to_string()),
                base_url: Some(google_compat_endpoint.to_string()),
                api_key: Some(google_api_key.clone()),
                preferred_model: google_config.preferred_model.clone(),
            };

            // 5. Create the OpenAICompatibleClient using the constructed config
            match OpenAICompatibleClient::new(compat_config_for_google) {
                Ok(client) => {
                    info!("AiClient (GoogleGemini): Successfully initialized using OpenAICompatibleClient via Google's compatible endpoint.");
                    Ok(Arc::new(client))
                }
                Err(e) => {
                    error!("AiClient (GoogleGemini): Failed to create OpenAICompatibleClient instance. Details: {}", e);
                    Err(e)
                }
            }
        }
        AiProvider::StepFunTTS => {
            info!("Attempting to initialize AiClient for StepFunTTS provider...");
            let stepfun_config = ai_config.stepfun_tts.as_ref().ok_or_else(|| {
                error!("AiClient (StepFunTTS): Initialization failed. The '[ai_settings.stepfun_tts]' configuration block is missing.");
                AiError::ClientError("StepFunTTS configuration is missing.".to_string())
            })?;
            match StepFunTtsClient::new(stepfun_config.clone()) {
                Ok(client) => {
                    info!("AiClient (StepFunTTS): Successfully initialized StepFunTtsClient. Note: This client is primarily for TTS and may have limited LLM capabilities via AiClient trait.");
                    Ok(Arc::new(client) as Arc<dyn AiClient + Send + Sync>)
                }
                Err(e) => {
                    error!("AiClient (StepFunTTS): Failed to create StepFunTtsClient instance. Details: {}", e);
                    Err(e)
                }
            }
        }
        AiProvider::OpenAICompatible => {
            info!("Attempting to initialize AiClient for OpenAICompatible provider...");
            let openai_config = config.ai_settings.openai_compatible.as_ref().ok_or_else(|| {
                 error!("AiClient (OpenAICompatible): Initialization failed. The '[ai_settings.openai_compatible]' configuration block is missing.");
                AiError::ClientError("OpenAICompatible configuration is missing.".to_string())
             })?;
             match OpenAICompatibleClient::new(openai_config.clone()) {
                 Ok(client) => {
                    info!("AiClient (OpenAICompatible): Successfully initialized OpenAICompatibleClient for provider: '{:?}'.", openai_config.name);
                    Ok(Arc::new(client))
                 }
                 Err(e) => {
                    error!("AiClient (OpenAICompatible): Failed to create OpenAICompatibleClient instance for provider '{:?}'. Details: {}", openai_config.name, e);
                    Err(e)
                 }
             }
        }
    }
} 