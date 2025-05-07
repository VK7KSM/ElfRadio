//! Implementation of the AuxServiceClient trait for Aliyun services.

use elfradio_types::{AiError, AliyunAuxCredentials, AuxServiceClient}; // Use correct type AliyunAuxCredentials
use elfradio_config::{get_user_config_value, ConfigError}; // Import ConfigError
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
        let access_key_id = get_user_config_value::<String>(key_path_id)
            .map_err(|e| AiError::Config(format!("Failed to read Aliyun Access Key ID: {}", e)))? // Map error
            .filter(|key| !key.is_empty()) // Ensure not empty
            .ok_or_else(|| {
                error!("Aliyun Access Key ID not found or empty in user configuration at key '{}'.", key_path_id);
                AiError::AuthenticationError(format!("Aliyun Access Key ID not found or empty (key: {}).", key_path_id))
            })?;

        // Read Access Key Secret from user config
        let access_key_secret = get_user_config_value::<String>(key_path_secret)
            .map_err(|e| AiError::Config(format!("Failed to read Aliyun Access Key Secret: {}", e)))? // Map error
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
        api_path: &str, // 应始终为 "/"，用于签名计算
        parameters: &BTreeMap<String, String> 
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

        // 3. Construct String-to-Sign
        // 修改：始终使用 "/" 作为签名路径
        let string_to_sign = format!(
             "{}&{}&{}",
             method.to_uppercase(),
             percent_encode("/"), // 固定使用 "/" 作为签名路径
             percent_encode(&canonicalized_query_string) 
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

// --- 添加开始: Aliyun Translate API 请求/响应结构体 ---

/// Represents the JSON body for the Aliyun TranslateGeneral API request.
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")] // Aliyun API uses PascalCase
struct AliyunTranslateRequest<'a> {
    format_type: &'static str, // Typically "text"
    source_language: &'a str,
    target_language: &'a str,
    source_text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    scene: Option<&'a str>, // e.g., "general"
}

/// Represents the nested 'Data' part of a successful Aliyun Translate response.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct AliyunTranslateResponseData {
    translated: String,
    // Add other fields from 'Data' if needed, e.g., DetectedLanguage, WordCount
}

/// Represents the overall structure of the Aliyun Translate API response.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct AliyunTranslateResponse {
    request_id: String,
    data: Option<AliyunTranslateResponseData>, // Present on success
    code: Option<String>,     // Present on success or failure
    message: Option<String>, // Present on success or failure
}
// --- 添加结束 ---

// Implement the trait with placeholder methods
#[async_trait]
impl AuxServiceClient for AliyunAuxClient {
    /// Translates text using Aliyun Machine Translation API (General Version).
    async fn translate(&self, text: &str, target_language: &str, source_language: Option<&str>) -> Result<String, AiError> {
        // API Details
        let action = "TranslateGeneral"; 
        let version = "2018-10-12";
        let endpoint = "mt.aliyuncs.com";
        let signing_path = "/"; // 修改：签名路径始终为 "/"
        let base_url = format!("https://{}", endpoint); // 修改：base_url 仅包含主机名
        let method = "POST";

        // 移除 request_body 的创建，将所有参数移至 params_for_sig
        let mut params_for_sig = BTreeMap::new();
        // Common Parameters
        params_for_sig.insert("Format".to_string(), "JSON".to_string());
        params_for_sig.insert("Version".to_string(), version.to_string());
        params_for_sig.insert("AccessKeyId".to_string(), self.access_key_id.clone());
        params_for_sig.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
        params_for_sig.insert("Timestamp".to_string(), Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        params_for_sig.insert("SignatureVersion".to_string(), "1.0".to_string());
        params_for_sig.insert("SignatureNonce".to_string(), Uuid::new_v4().to_string().replace('-', ""));

        // Action Specific Parameters - 移至参数映射中
        params_for_sig.insert("Action".to_string(), action.to_string());
        params_for_sig.insert("SourceLanguage".to_string(), source_language.unwrap_or("auto").to_string());
        params_for_sig.insert("TargetLanguage".to_string(), target_language.to_string());
        params_for_sig.insert("SourceText".to_string(), text.to_string());
        params_for_sig.insert("FormatType".to_string(), "text".to_string());
        params_for_sig.insert("Scene".to_string(), "general".to_string());

        // 使用 signing_path 计算签名
        let signature = self.calculate_aliyun_signature(method, signing_path, &params_for_sig)?;

        // 将签名添加到参数中，然后生成查询字符串
        params_for_sig.insert("Signature".to_string(), signature);
        let query_string = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params_for_sig.iter())
            .finish();

        // 构造最终 URL（包含查询字符串）
        let final_url = format!("{}?{}", base_url, query_string);
        debug!("Sending translation request to Aliyun API Endpoint: {}", final_url);

        // 发送请求（使用空请求体）
        let response = self.http_client
            .post(&final_url)
            .header(ACCEPT, "application/json")
            .body("") // 空请求体，所有参数已在查询字符串中
            .send()
            .await
            .map_err(|e| {
                 error!("Aliyun network request failed: {}", e);
                 AiError::RequestError(format!("Aliyun network request failed: {}", e))
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

        // Check for API-level errors indicated in the response body
        if let Some(code) = response_body.code {
             if code != "200" { // Assuming "200" is the success code for Aliyun Translate
                 let message = response_body.message.unwrap_or_else(|| "No error message provided.".to_string());
                 error!("Aliyun API Error: Code={}, Message={}", code, message);
                 // Map to AiError::ApiError
                 return Err(AiError::ApiError {
                     status: status.as_u16(), // Keep HTTP status if available
                     message: format!("Aliyun API Error (Code: {}): {}", code, message),
                 });
             }
        }

        // If we have a 'Data' field and HTTP status is success, extract translation
        if status.is_success() {
        if let Some(data) = response_body.data {
                debug!("Aliyun translation successful (RequestId: {}).", response_body.request_id);
            Ok(data.translated)
            } else {
                // Success status but no data field - treat as an API error or unexpected response
                error!("Aliyun API returned success status ({}) but no 'Data' field in response. RequestId: {}", status, response_body.request_id);
                Err(AiError::ApiError {
                    status: status.as_u16(),
                    message: format!("Aliyun API Success status ({}) but no translation data received (RequestId: {}).", status, response_body.request_id),
                })
            }
        } else {
             // Handle non-success HTTP status codes even if code/message wasn't in JSON
             let message = response_body.message.unwrap_or_else(|| response_text); // Use body text if no message field
             error!("Aliyun HTTP Error: Status={}, Body={}", status, message);
             Err(AiError::ApiError {
                 status: status.as_u16(),
                 message: format!("Aliyun HTTP Error {}: {}", status.as_u16(), message),
             })
        }
    }

    /// Converts text to speech audio (Placeholder).
    async fn text_to_speech(&self, _text: &str, _language_code: &str, _voice_name: Option<&str>) -> Result<Vec<u8>, AiError> {
        warn!("AliyunAuxClient::text_to_speech is not implemented.");
        // Use NotSupported instead of NotImplemented
        Err(AiError::NotSupported("Aliyun TTS not yet implemented".to_string()))
    }

    /// Converts speech audio to text (Placeholder).
    async fn speech_to_text(&self, _audio_data: &[u8], _sample_rate_hertz: u32, _language_code: &str) -> Result<String, AiError> {
        warn!("AliyunAuxClient::speech_to_text is not implemented.");
        // Use NotSupported instead of NotImplemented
        Err(AiError::NotSupported("Aliyun STT not yet implemented".to_string()))
    }
} 