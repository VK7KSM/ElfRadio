use axum::{
    extract::{State, Json},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use base64::{Engine as _, engine::general_purpose::STANDARD as Base64Standard}; // For base64 decoding
use serde_json::json; // Ensure serde_json::json macro is imported

use elfradio_core::AppState;
use elfradio_types::{
    AiError as ElfAiError,
    TestTtsRequest,
    TestSttRequest,
};
use crate::error::ApiError; // Local ApiError

/// Helper function to map elfradio_types::AiError to crate::error::ApiError
// This function should ideally live in error.rs or a shared place if used by many handlers.
// For now, defining it here for clarity for these specific test handlers.
fn map_elf_ai_error_to_api_error(ai_error: ElfAiError) -> ApiError {
    match ai_error {
        ElfAiError::AuthenticationError(msg) => {
            warn!("AuthenticationError from AuxService: {}", msg);
            ApiError::Unauthorized(format!("Auxiliary service authentication failed: {}", msg))
        }
        ElfAiError::ApiError { status, message } => {
            error!("ApiError from AuxService: Status {}, Message {}", status, message);
            ApiError::BadGateway(status, format!("Auxiliary service API error: {}", message))
        }
        ElfAiError::Config(msg) => {
            error!("Configuration error from AuxService: {}", msg);
            ApiError::InternalServerError(format!("Auxiliary service configuration error: {}", msg))
        }
        ElfAiError::ClientError(msg) => {
            error!("ClientError from AuxService: {}", msg);
            ApiError::InternalServerError(format!("Auxiliary service client error: {}", msg))
        }
        ElfAiError::RequestError(msg) => {
            error!("RequestError from AuxService: {}", msg);
            ApiError::InternalServerError(format!("Auxiliary service request error: {}", msg))
        }
        ElfAiError::ResponseParseError(msg) => {
            error!("ResponseParseError from AuxService: {}", msg);
            ApiError::BadGateway(502, format!("Auxiliary service response parse error: {}", msg)) // Use 502 for bad gateway
        }
        ElfAiError::Audio(msg) | ElfAiError::AudioDecodingError(msg) => {
            error!("Audio processing/decoding error from AuxService: {}", msg);
            ApiError::InternalServerError(format!("Auxiliary service audio error: {}", msg))
        }
        ElfAiError::NotSupported(msg) => {
            warn!("OperationNotSupported by AuxService: {}", msg);
            ApiError::ServiceUnavailable(format!("Auxiliary service operation not supported: {}", msg))
        }
        ElfAiError::InvalidInput(msg) => {
            warn!("InvalidInput for AuxService: {}", msg);
            ApiError::BadRequest(format!("Invalid input for auxiliary service: {}", msg))
        }
        ElfAiError::ProviderNotSpecified => {
            error!("Auxiliary service provider not specified.");
            ApiError::InternalServerError("Auxiliary service provider not specified.".to_string())
        }
        ElfAiError::Unknown => {
            error!("Unknown error from AuxService.");
            ApiError::InternalServerError("Unknown auxiliary service error.".to_string())
        }
    }
}

/// Handler for testing Text-to-Speech functionality.
pub async fn test_tts_handler(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<TestTtsRequest>,
) -> Result<Response, ApiError> {
    info!("Received TTS test request: {:?}", payload);

    let aux_client_guard = app_state.aux_client.read().await;
    if let Some(client) = aux_client_guard.as_ref() {
        match client.text_to_speech(&payload.text, &payload.language_code, payload.voice_name.as_deref()).await {
            Ok(audio_bytes) => {
                info!("TTS test successful, returning {} audio bytes.", audio_bytes.len());
                Ok((
                    StatusCode::OK, // Explicitly set status code
                    [(header::CONTENT_TYPE, "audio/wav")],
                    audio_bytes
                ).into_response())
            }
            Err(ai_error) => {
                error!("TTS test failed: {:?}", ai_error);
                Err(map_elf_ai_error_to_api_error(ai_error))
            }
        }
    } else {
        error!("Auxiliary TTS service not configured for TTS test.");
        Err(ApiError::ServiceUnavailable("Auxiliary TTS service not configured.".to_string()))
    }
}

/// Handler for testing Speech-to-Text functionality.
pub async fn test_stt_handler(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<TestSttRequest>,
) -> Result<Json<serde_json::Value>, ApiError> { // Return Json<serde_json::Value>
    info!("Received STT test request: lang={}, sample_rate={}", payload.language_code, payload.sample_rate_hertz);

    let audio_bytes = Base64Standard.decode(&payload.audio_base64)
        .map_err(|e| {
            error!("Failed to decode base64 audio for STT test: {}", e);
            ApiError::BadRequest(format!("Invalid base64 audio data: {}", e))
        })?;

    debug!("Decoded {} bytes of audio data for STT test.", audio_bytes.len());

    let aux_client_guard = app_state.aux_client.read().await;
    if let Some(client) = aux_client_guard.as_ref() {
        match client.speech_to_text(&audio_bytes, payload.sample_rate_hertz, &payload.language_code).await {
            Ok(transcript) => {
                info!("STT test successful. Transcript: {}...", transcript.chars().take(50).collect::<String>());
                Ok(Json(json!({ "status": "success", "transcript": transcript }))) // Use json! macro
            }
            Err(ai_error) => {
                error!("STT test failed: {:?}", ai_error);
                Err(map_elf_ai_error_to_api_error(ai_error))
            }
        }
    } else {
        error!("Auxiliary STT service not configured for STT test.");
        Err(ApiError::ServiceUnavailable("Auxiliary STT service not configured.".to_string()))
    }
} 