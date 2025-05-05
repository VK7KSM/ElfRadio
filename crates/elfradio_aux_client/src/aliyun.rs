//! Implementation of the AuxServiceClient trait for Aliyun services.

use elfradio_types::{AiError, AliyunAuxConfig, AuxServiceClient}; // Import config, error, and trait
use elfradio_config::get_user_config_value;
use reqwest::{Client as ReqwestClient, header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, DATE}}; // Add header imports
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error, debug, warn, trace}; // Added debug, warn, and trace
use async_trait::async_trait; // For trait implementation
use chrono::{Utc, SecondsFormat}; // For timestamp in signature
use hmac::{Hmac, Mac}; // For HMAC calculation
use sha1::Sha1; // For SHA1 hash
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _}; // For Base64 encoding signature
use url::form_urlencoded; // For canonical query string
use uuid::Uuid; // For Nonce
use std::collections::BTreeMap; // For sorting parameters
use std::time::Duration; // Added for client timeout

#[derive(Debug, Clone)]
pub struct AliyunAuxClient {
    access_key_id: String,
    access_key_secret: String,
    http_client: ReqwestClient,
    // region_id: String, // Store region if needed
}

impl AliyunAuxClient {
    /// Creates a new Aliyun Auxiliary Client.
    /// Reads the AccessKey ID and Secret from the user's configuration file.
    pub async fn new() -> Result<Self, AiError> {
        info!("Initializing AliyunAuxClient...");

        // Corrected config key path assuming it's under aux_service_settings.aliyun
        let key_path_id = "aux_service_settings.aliyun.access_key_id";
        let key_path_secret = "aux_service_settings.aliyun.access_key_secret";

        // Read Access Key ID from user config
        let access_key_id = get_user_config_value::<String>(key_path_id)?
            .filter(|key| !key.is_empty()) // Ensure not empty
            .ok_or_else(|| {
                error!("Aliyun Access Key ID not found or empty in user configuration at key '{}'.", key_path_id);
                AiError::AuthenticationError(format!("Aliyun Access Key ID not found or empty (key: {}).", key_path_id))
            })?;

        // Read Access Key Secret from user config
        let access_key_secret = get_user_config_value::<String>(key_path_secret)?
            .filter(|secret| !secret.is_empty()) // Ensure not empty
            .ok_or_else(|| {
                error!("Aliyun Access Key Secret not found or empty in user configuration at key '{}'.", key_path_secret);
                AiError::AuthenticationError(format!("Aliyun Access Key Secret not found or empty (key: {}).", key_path_secret))
            })?;

        // Read Region ID (optional example)
        // let region_id = get_user_config_value::<String>("aux_service_settings.aliyun.region_id")?
        //     .unwrap_or_else(|| "cn-shanghai".to_string()); // Default region

        info!("Successfully retrieved Aliyun credentials from user config.");

        // Consider adding timeouts and other client configurations
        let http_client = ReqwestClient::builder()
            .timeout(Duration::from_secs(30)) // Example timeout
            .build()
            .map_err(|e| AiError::ClientError(format!("Failed to build Reqwest client: {}", e)))?;

        Ok(Self {
            access_key_id,
            access_key_secret,
            http_client,
            // region_id,
        })
    }

    /// Calculates the HMAC-SHA1 signature for Aliyun API requests.
    /// CRITICAL: The exact format of the string_to_sign MUST be verified against
    /// the specific Aliyun API documentation (e.g., Machine Translation).
    /// The current implementation uses a common simplified format for signing query parameters.
    fn calculate_aliyun_signature(
        &self,
        method: &str, // "GET" or "POST"
        api_path: &str, // e.g., "/api/translate/web/general"
        parameters: &BTreeMap<String, String> // Sorted map of parameters included in the signature (usually common + action)
    ) -> Result<String, AiError> {
        // 1. Percent-encode keys and values according to RFC3986
        let mut encoded_pairs: Vec<String> = Vec::new();
        for (key, value) in parameters {
            let encoded_key = percent_encode(key);
            let encoded_value = percent_encode(value);
            encoded_pairs.push(format!("{}={}", encoded_key, encoded_value));
        }
        // Sorting happens implicitly via BTreeMap iteration

        // 2. Construct Canonicalized Query String from parameters to be signed
        let canonicalized_query_string = encoded_pairs.join("&");
        trace!("Canonicalized Query String for Signature: {}", canonicalized_query_string);

        // 3. Construct String-to-Sign (VERIFY THIS FORMAT WITH ALIYUN MACHINE TRANSLATION DOCS)
        // Format: METHOD + "&" + PercentEncode(Path) + "&" + PercentEncode(CanonicalQueryString)
        let string_to_sign = format!(
             "{}&{}&{}",
             method.to_uppercase(),
             percent_encode(api_path), // Use the provided API path
             percent_encode(&canonicalized_query_string) // Percent-encoded canonical query string
         );

        trace!("String-to-Sign: {}", string_to_sign);

        // 4. Calculate HMAC-SHA1
        type HmacSha1 = Hmac<Sha1>;
        let secret_with_ampersand = format!("{}&", self.access_key_secret); // Append "&" to secret key
        let mut mac = HmacSha1::new_from_slice(secret_with_ampersand.as_bytes())
            .map_err(|e| AiError::ClientError(format!("Failed to create HMAC-SHA1 instance: {}", e)))?;
        mac.update(string_to_sign.as_bytes());
        let signature_bytes = mac.finalize().into_bytes();

        // 5. Base64 Encode Signature
        let signature = BASE64_STANDARD.encode(signature_bytes);
        trace!("Calculated Signature: {}", signature);

        Ok(signature)
    }

    // Placeholder for request sending helper
    // async fn send_aliyun_request<T: serde::de::DeserializeOwned>(&self, ...) -> Result<T, AiError> { ... }
}

// Helper function for Aliyun-specific percent encoding
// Based on https://help.aliyun.com/document_detail/29475.html
// Encodes non-alphanumeric characters except '-', '_', '.', '~'. Encodes space as %20.
fn percent_encode(input: &str) -> String {
    form_urlencoded::byte_serialize(input.as_bytes())
        .collect::<String>()
        // Aliyun docs specify space should be %20, which byte_serialize does.
        // They also mention '~' should NOT be encoded, which byte_serialize handles.
        // '*' should be encoded as %2A, which byte_serialize also handles.
        // Let's double-check '+' encoding if using GET later (should be %2B, not space).
}

// Implement the trait with placeholder methods
#[async_trait]
impl AuxServiceClient for AliyunAuxClient {
    /// Translates text using Aliyun Machine Translation API (General Version).
    async fn translate(&self, text: &str, target_language: &str, source_language: Option<&str>) -> Result<String, AiError> {
        // API Details - VERIFY THESE AGAINST OFFICIAL DOCS
        let action = "TranslateGeneral"; // API Action name
        let version = "2018-10-12";       // API Version for general translation
        let endpoint = "mt.aliyuncs.com"; // General endpoint, consider region if needed (e.g., mt.cn-hangzhou.aliyuncs.com)
        let api_path = "/api/translate/web/general"; // Specific path for the general web translation API
        let base_url = format!("https://{}{}", endpoint, api_path);
        let method = "POST";

        // 1. Prepare Request Body (Parameters specific to the action)
        let request_body = AliyunTranslateRequest {
            format_type: "text",
            source_language: source_language.unwrap_or("auto"),
            target_language,
            source_text: text,
            scene: Some("general"), // Or make configurable/optional if needed
        };
        trace!("Aliyun Translate Request Body: {:?}", request_body);

        // 2. Prepare Common & Action Parameters for Signature (Sorted)
        // These go into the query string part of the request and are signed.
        let mut params_for_sig = BTreeMap::new();
        // Common Parameters
        params_for_sig.insert("Format".to_string(), "JSON".to_string());
        params_for_sig.insert("Version".to_string(), version.to_string());
        params_for_sig.insert("AccessKeyId".to_string(), self.access_key_id.clone());
        params_for_sig.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
        params_for_sig.insert("Timestamp".to_string(), Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        params_for_sig.insert("SignatureVersion".to_string(), "1.0".to_string());
        params_for_sig.insert("SignatureNonce".to_string(), Uuid::new_v4().to_string().replace('-', "")); // Unique nonce
        // Add region if needed for endpoint/signing: params_for_sig.insert("RegionId".to_string(), "cn-hangzhou".to_string());
        // Action Parameter (included in query string for signing)
        params_for_sig.insert("Action".to_string(), action.to_string());

        // 3. Calculate Signature using parameters intended for query string
        let signature = self.calculate_aliyun_signature(method, api_path, ¶ms_for_sig)?;

        // 4. Build Final Query String with Signature
        params_for_sig.insert("Signature".to_string(), signature); // Add signature to the map
        let query_string = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params_for_sig.iter()) // URL-encode the parameters
            .finish();

        // 5. Construct Final URL
        let final_url = format!("{}?{}", base_url, query_string);
        debug!("Sending translation request to Aliyun API Endpoint: {}", final_url);

        // 6. Send Request (POST with JSON body and signature in query params)
        let response = self.http_client
            .post(&final_url) // URL contains common params + signature
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json") // Body is JSON
            .json(&request_body) // Send the action-specific params as JSON body
            .send()
            .await
            .map_err(|e| {
                 error!("Aliyun network request failed: {}", e);
                 AiError::NetworkError(format!("Aliyun network request failed: {}", e))
            })?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
            error!("Failed to read Aliyun response body: {}", e);
            AiError::ResponseParseError(format!("Failed to read Aliyun response body: {}", e))
        })?;
        trace!("Aliyun Translate Response Status: {}, Body: {}", status, response_text);

        // 7. Parse Response (Handle both Success and API Error responses)
        let response_body: AliyunTranslateResponse = match serde_json::from_str(&response_text) {
            Ok(body) => body,
            Err(e) => {
                error!("Failed to parse Aliyun response JSON: {}. Body: {}", e, response_text);
                // Include status code in the error if it's not success
                if !status.is_success() {
                     return Err(AiError::ApiError{ status: status.as_u16(), message: format!("HTTP Error {}. Failed to parse Aliyun response JSON: {}. Body: {}", status.as_u16(), e, response_text)});
                } else {
                     return Err(AiError::ResponseParseError(format!("Failed to parse Aliyun response JSON: {}. Body: {}", e, response_text)));
                }
            }
        };

        // Check for explicit API errors in the response body
        // Aliyun might return HTTP 200 but have an error code in the body
        if let Some(code) = &response_body.code {
             // Based on docs, a successful response might not have a 'Code' field, or it might be specific like "Success.NotUpdate"
             // Let's assume any presence of 'Code' that isn't clearly success indicates an issue, or if HTTP status wasn't 200.
             // A more robust check would compare against known success codes.
             if !status.is_success() || (code != "200" && !code.starts_with("Success")) { // Example check, VERIFY success codes
                 let message = response_body.message.as_deref().unwrap_or("Unknown Aliyun API error");
                 warn!("Aliyun Translate API returned error. Code: {}, Message: {}", code, message);
                 // Return ApiError using the parsed code/message
                 return Err(AiError::ApiError { status: status.as_u16(), message: format!("Code: {}, Message: {}", code, message) });
             }
        } else if !status.is_success() {
            // Handle non-success HTTP status when Code field is missing
             warn!("Aliyun Translate API HTTP error. Status: {}, Body: {}", status, response_text);
             let message = response_body.message.unwrap_or(response_text); // Fallback
             return Err(AiError::ApiError { status: status.as_u16(), message });
        }


        // Extract translated text from successful response data
        if let Some(data) = response_body.data {
            debug!("Aliyun Translation successful. RequestId: {}", response_body.request_id);
            Ok(data.translated)
        } else {
            // This case might happen if HTTP 200 is returned but 'Data' is missing
            let err_msg = format!("'Data' field missing in successful Aliyun response (HTTP {}). RequestId: {}. Body: {}", status, response_body.request_id, response_text);
            error!("{}", err_msg);
            Err(AiError::ResponseParseError(err_msg))
        }
    }

    async fn text_to_speech(&self, _text: &str, _language_code: &str, _voice_name: Option<&str>) -> Result<Vec<u8>, AiError> {
        warn!("Aliyun text_to_speech called but not implemented.");
        Err(AiError::NotImplemented("Aliyun TTS not yet implemented".to_string()))
    }

    async fn speech_to_text(&self, _audio_data: &[u8], _sample_rate_hertz: u32, _language_code: &str) -> Result<String, AiError> {
        warn!("Aliyun speech_to_text called but not implemented.");
        Err(AiError::NotImplemented("Aliyun STT not yet implemented".to_string()))
    }
} 