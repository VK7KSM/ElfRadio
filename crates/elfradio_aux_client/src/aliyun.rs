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
use serde_json; // Added for direct JSON parsing
use tokio::sync::Mutex; // Added for Mutex
use chrono::DateTime; // Added for DateTime, Utc and Duration are already imported via `chrono::{Utc, SecondsFormat, Duration}` if Duration is used below. Will ensure `chrono::Duration` is explicitly available.

#[derive(Debug)] // Removed Clone as Mutex is not Clone
pub struct AliyunAuxClient {
    access_key_id: String,
    access_key_secret: String,
    app_key: String,
    http_client: ReqwestClient,
    cached_token: Mutex<Option<(String, DateTime<Utc>)>>, // Added cached_token field
    // region_id: String, // Store region if needed
}

impl AliyunAuxClient {
    /// Creates a new Aliyun Auxiliary Client.
    /// Reads the AccessKey ID, Secret, and AppKey from the user's configuration file.
    pub async fn new() -> Result<Self, AiError> {
        info!("Initializing AliyunAuxClient...");

        // Corrected config key path assuming it's under aux_service_settings.aliyun
        let key_path_id = "aux_service_settings.aliyun.access_key_id";
        let key_path_secret = "aux_service_settings.aliyun.access_key_secret";
        let key_path_app_key = "aux_service_settings.aliyun.app_key";

        // Read Access Key ID from user config
        let access_key_id = get_user_config_value::<String>(key_path_id)
            .map_err(|e| AiError::Config(format!("Failed to read Aliyun Access Key ID: {}", e)))?
            .filter(|key| !key.is_empty())
            .ok_or_else(|| {
                error!("Aliyun Access Key ID not found or empty in user configuration at key '{}'.", key_path_id);
                AiError::AuthenticationError(format!("Aliyun Access Key ID not found or empty (key: {}).", key_path_id))
            })?;

        // Read Access Key Secret from user config
        let access_key_secret = get_user_config_value::<String>(key_path_secret)
            .map_err(|e| AiError::Config(format!("Failed to read Aliyun Access Key Secret: {}", e)))?
            .filter(|secret| !secret.is_empty())
            .ok_or_else(|| {
                error!("Aliyun Access Key Secret not found or empty in user configuration at key '{}'.", key_path_secret);
                AiError::AuthenticationError(format!("Aliyun Access Key Secret not found or empty (key: {}).", key_path_secret))
            })?;

        // Read AppKey from user config
        let app_key = match get_user_config_value::<String>(key_path_app_key) {
            Ok(Some(key)) if !key.is_empty() => key,
            Ok(Some(_)) | Ok(None) => {
                let err_msg = format!("Aliyun AppKey not found or empty in user configuration at key '{}'. TTS functionality will be unavailable.", key_path_app_key);
                error!("{}", err_msg);
                return Err(AiError::AuthenticationError(format!("Aliyun AppKey not found or empty (key: {}). Required for TTS.", key_path_app_key)));
            }
            Err(e) => {
                let err_msg = format!("Failed to read Aliyun AppKey from user config (key: {}): {}", key_path_app_key, e);
                error!("{}", err_msg);
                return Err(AiError::Config(format!("Failed to read Aliyun AppKey: {}", e)));
            }
        };

        info!("Successfully retrieved Aliyun credentials (including AppKey) from user config.");

        let http_client = ReqwestClient::builder()
            .timeout(std::time::Duration::from_secs(30)) // Using std::time::Duration fully qualified
            .build()
            .map_err(|e| AiError::ClientError(format!("Failed to build Reqwest client: {}", e)))?;

        Ok(Self {
            access_key_id,
            access_key_secret,
            app_key,
            http_client,
            cached_token: Mutex::new(None), // Initialize cached_token
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

    /// Fetches an AccessToken from Aliyun's CreateToken API.
    /// This token is typically short-lived and used for NLS (Natural Language Service) APIs like TTS.
    async fn get_access_token(&self) -> Result<(String, i64), AiError> {
        debug!("Attempting to get Aliyun AccessToken...");

        // a. Define API Endpoint and Method
        let endpoint_host = "nlsmeta.ap-southeast-1.aliyuncs.com"; // 修改: 更新为正确的端点主机
        let api_protocol = "http"; // Ensure HTTPS
        let http_method = "POST"; // Changed to POST
        let base_url = format!("{}://{}", api_protocol, endpoint_host);

        // b. Prepare Common Parameters for CreateToken
        let mut common_params = BTreeMap::new();
        common_params.insert("Action".to_string(), "CreateToken".to_string());
        common_params.insert("Version".to_string(), "2019-07-17".to_string()); // Added Version
        common_params.insert("Format".to_string(), "JSON".to_string());
        common_params.insert("AccessKeyId".to_string(), self.access_key_id.clone());
        common_params.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
        common_params.insert("Timestamp".to_string(), Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        common_params.insert("SignatureVersion".to_string(), "1.0".to_string());
        common_params.insert("SignatureNonce".to_string(), Uuid::new_v4().to_string().replace('-', ""));
        common_params.insert("RegionId".to_string(), "ap-southeast-1".to_string()); // Added RegionId
        
        trace!("Common Parameters for CreateToken: {:?}", common_params);

        // c. Calculate Signature
        let signature = self.calculate_aliyun_signature(http_method, "/", &common_params)?;
        trace!("Calculated signature for CreateToken: {}", signature);

        // d. Construct Final Request URL (parameters are in query string for this RPC style)
        let mut request_params = common_params;
        request_params.insert("Signature".to_string(), signature);

        let query_string = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(request_params.iter())
            .finish();
        
        let final_url = format!("{}?{}", base_url, query_string); // Parameters remain in URL for RPC POST
        debug!("Final URL for CreateToken POST request: {}", final_url);

        // e. Send HTTP Request
        let response = self.http_client
            .post(&final_url) // Changed to POST
            .header(ACCEPT, "application/json")
            // For POST with empty body and params in URL query string.
            // Some APIs might require Content-Type: application/x-www-form-urlencoded even with empty body.
            // Let's explicitly set Content-Length to 0 by sending an empty body.
            .body("".to_string()) // Explicitly empty body for POST
            .send()
            .await
            .map_err(|e| {
                error!("CreateToken network POST request failed: {}", e);
                AiError::RequestError(format!("CreateToken network POST request failed: {}", e))
            })?;

        // f. Process Response
        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
            error!("Failed to read CreateToken response body: {}", e);
            AiError::ResponseParseError(format!("Failed to read CreateToken response body: {}", e))
        })?;
        trace!("CreateToken Response Status: {}, Body: {}", status, response_text);

        // Attempt to parse the response regardless of HTTP status, as error details might be in JSON body
        match serde_json::from_str::<AliyunCreateTokenResponse>(&response_text) {
            Ok(parsed_response) => {
                // Check if HTTP status is successful AND token is present
                if status.is_success() && parsed_response.token.is_some() {
                    let token_data = parsed_response.token.unwrap(); // Safe due to check
                    if token_data.id.is_empty() {
                        error!(
                            "Aliyun CreateToken API Error: Received empty Token ID. RequestId: {:?}, NlsRequestId: {:?}. Full Body: {}",
                            parsed_response.request_id,
                            parsed_response.nls_request_id,
                            response_text
                        );
                        return Err(AiError::ApiError {
                            status: status.as_u16(),
                            message: format!(
                                "Aliyun CreateToken API Error: Received empty Token ID. RequestId: {:?}, Full Body: {}",
                                parsed_response.request_id, response_text
                            ),
                        });
                    }

                    // Log if an error message was present alongside a token (unusual but possible)
                    if parsed_response.err_code.is_some() || parsed_response.err_msg.is_some() {
                        warn!(
                            "Aliyun CreateToken API returned a token but also an error indication (ErrCode: {:?}, ErrMsg: {:?}). Proceeding with token. RequestId: {:?}, NlsRequestId: {:?}. Full Body: {}",
                            parsed_response.err_code, parsed_response.err_msg, parsed_response.request_id, parsed_response.nls_request_id, response_text
                        );
                    }
                    
                    // --- ADD LOGGING HERE ---
                    info!(
                        "Aliyun CreateToken Success: Fetched Token ID: '{}', ExpireTime (Unix timestamp): {}, UserId: '{}'. RequestId: {:?}, NlsRequestId: {:?}",
                        token_data.id,
                        token_data.expire_time,
                        token_data.user_id, // Also log UserId for completeness
                        parsed_response.request_id,
                        parsed_response.nls_request_id
                    );
                    // --- END LOGGING ---

                    debug!("Successfully obtained Aliyun AccessToken. RequestId: {:?}, NisRequestId: {:?}", parsed_response.request_id, parsed_response.nls_request_id);
                    Ok((token_data.id, token_data.expire_time))
                } else {
                    // HTTP status might be success but no token, or HTTP status is error.
                    // Log detailed error from parsed response.
                    let api_err_msg = parsed_response.err_msg.as_deref().unwrap_or("N/A");
                    let api_err_code = parsed_response.err_code.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string());
                    
                    error!(
                        "Aliyun CreateToken API call failed or returned no Token. HTTP Status: {}. \
                        API Response Details: RequestId='{:?}', NlsRequestId='{:?}', ErrMsg='{}', ErrCode='{}'. \
                        Full Body: {}",
                        status,
                        parsed_response.request_id,
                        parsed_response.nls_request_id,
                        api_err_msg,
                        api_err_code,
                        response_text
                    );
                    Err(AiError::ApiError {
                        status: status.as_u16(),
                        message: format!(
                            "Aliyun CreateToken failed. HTTP Status: {}. API Msg: '{}', API Code: '{}', RequestId: '{:?}'. Full Response: {}",
                            status,
                            api_err_msg,
                            api_err_code,
                            parsed_response.request_id,
                            response_text
                        ),
                    })
                }
            }
            Err(parse_err) => {
                // JSON parsing failed. Log the raw response text.
                error!(
                    "Failed to parse Aliyun CreateToken response JSON. HTTP Status: {}. Parse Error: {}. Body: {}",
                    status, parse_err, response_text
                );
                Err(AiError::ResponseParseError(format!(
                    "Failed to parse CreateToken response JSON (HTTP Status {}): {}. Body: {}",
                    status, parse_err, response_text
                )))
            }
        }
    }

    /// Ensures a valid AccessToken is available, by checking the cache or fetching a new one.
    pub async fn ensure_valid_token(&self) -> Result<String, AiError> {
        let mut cached_token_guard = self.cached_token.lock().await; // Acquire lock

        if let Some((token, expires_at)) = cached_token_guard.as_ref() {
            // Using chrono::Duration directly
            let refresh_buffer = chrono::Duration::minutes(5); // Refresh 5 minutes before actual expiry
            if Utc::now() < (*expires_at - refresh_buffer) {
                debug!("Using valid cached Aliyun AccessToken, expires at: {}", expires_at.to_rfc3339());
                // --- ADD LOGGING HERE ---
                info!("Using cached Aliyun Token ID: '{}', Expires At (UTC): {}", token, expires_at.to_rfc3339());
                // --- END LOGGING ---
                return Ok(token.clone()); // Return cloned token
            } else {
                info!("Cached Aliyun AccessToken expired or nearing expiry (expires at: {}). Refreshing...", expires_at.to_rfc3339());
            }
        } else {
            info!("No Aliyun AccessToken in cache. Fetching a new one...");
        }

        // If we reach here, token is not present, expired, or needs refresh.
        // The lock is still held.

        debug!("Calling private get_access_token method to fetch/refresh token...");
        let (new_token_id, new_expire_timestamp_secs) = self.get_access_token().await?;
        
        // Convert Unix timestamp (seconds) to DateTime<Utc>
        let new_expires_at_utc = DateTime::from_timestamp(new_expire_timestamp_secs, 0)
            .ok_or_else(|| {
                let err_msg = format!("Invalid token expiry timestamp received from Aliyun: {}. Cannot convert to DateTime<Utc>.", new_expire_timestamp_secs);
                error!("{}", err_msg);
                AiError::ClientError(err_msg)
            })?;

        // --- MODIFY EXISTING LOGGING HERE ---
        info!(
            "Successfully fetched new Aliyun AccessToken. ID: '{}', Expires At (UTC): {}, Original ExpireTime (Unix): {}",
            new_token_id,
            new_expires_at_utc.to_rfc3339(),
            new_expire_timestamp_secs // Log the raw timestamp too
        );
        // --- END MODIFIED LOGGING ---

        *cached_token_guard = Some((new_token_id.clone(), new_expires_at_utc));
        
        Ok(new_token_id)
        // MutexGuard is dropped here when it goes out of scope, releasing the lock.
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

// --- Aliyun CreateToken API Response Structures (Refactored) ---
#[derive(Deserialize, Debug, Clone)]
pub struct AliyunTokenData {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "ExpireTime")]
    pub expire_time: i64,
    #[serde(rename = "UserId")] // Based on api-docs.json
    pub user_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AliyunCreateTokenResponse {
    #[serde(rename = "RequestId")]
    pub request_id: Option<String>, // Changed to Option<String>
    #[serde(rename = "NlsRequestId")]
    pub nls_request_id: Option<String>, // Already Option<String>, which is good
    #[serde(rename = "Token")]
    pub token: Option<AliyunTokenData>,
    #[serde(rename = "ErrMsg")]
    pub err_msg: Option<String>,
    #[serde(rename = "ErrCode")]
    pub err_code: Option<i32>,
    // Added top-level Code and Message fields as optional
    #[serde(rename = "Code")]
    pub code: Option<String>,
    #[serde(rename = "Message")]
    pub message: Option<String>,
}

// --- End Aliyun CreateToken API Response Structures ---

// --- Aliyun ASR (一句话识别) API Response Structures (Simplified) ---

/// Represents the overall JSON response from Aliyun's ASR API.
#[derive(Deserialize, Debug, Clone)]
pub struct AliyunAsrResponse {
    #[serde(alias = "RequestId")] 
    pub request_id: Option<String>,

    #[serde(rename = "TaskId")]
    pub task_id: Option<String>,

    #[serde(rename = "StatusText")] // Maps to JSON "StatusText", e.g., "TranscriptionCompleted" or "SUCCESS" from message
    pub status_text: Option<String>,
    
    #[serde(rename = "status")]     // Maps to JSON "status", e.g., 20000000 for success
    pub status_code_val: Option<i64>, 

    #[serde(rename = "result")]     // Maps to JSON "result" which directly contains the transcription string
    pub result: Option<String>,

    #[serde(rename = "Code")]       // General API code, might be string like "200" or error codes
    pub code: Option<String>,

    #[serde(rename = "Message")]    // General API message, e.g., "SUCCESS" or error details
    pub message: Option<String>,

    // Specific error fields, snake_case as per previous observation
    pub error_code: Option<String>, 
    pub error_message: Option<String>,
}
// --- End Aliyun ASR API Response Structures ---

// --- Aliyun Translate API 请求/响应结构体 ---

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
// --- End Aliyun Translate API Response Structures ---

// Implement the trait with placeholder methods
#[async_trait]
impl AuxServiceClient for AliyunAuxClient {
    /// Translate text using Aliyun Machine Translation.
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
            .body("".to_string()) // Ensure body is a compatible type e.g. String or Vec<u8> for POST
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

    /// Synthesize speech from text using Aliyun NLS TTS.
    async fn text_to_speech(&self, text: &str, _language_code: &str, voice_name: Option<&str>) -> Result<Vec<u8>, AiError> {
        debug!("AliyunAuxClient: text_to_speech called. Text length: {}, Voice: {:?}", text.len(), voice_name);

        if self.app_key.is_empty() {
            error!("Aliyun AppKey is not configured. TTS functionality is unavailable.");
            return Err(AiError::AuthenticationError("Aliyun AppKey not configured for TTS.".to_string()));
        }

        let token = self.ensure_valid_token().await.map_err(|e| {
            error!("Failed to ensure valid Aliyun token for TTS: {:?}", e);
            e 
        })?;
        trace!("Aliyun TTS: Using token: {}...", &token[..std::cmp::min(10, token.len())]); // Log only a prefix of the token

        let selected_voice = voice_name.filter(|v_name| !v_name.is_empty()).unwrap_or("Aiyue");
        trace!("Aliyun TTS: Selected voice: {}", selected_voice);

        let mut query_params = Vec::new();
        query_params.push(("appkey", self.app_key.as_str()));
        query_params.push(("token", token.as_str()));
        query_params.push(("text", text));
        query_params.push(("format", "pcm")); // 修改：从 "wav" 改为 "pcm"
        query_params.push(("sample_rate", "16000"));
        query_params.push(("voice", selected_voice));
        // Optional params like volume, speech_rate, pitch_rate will use API defaults if not added here.

        let query_string = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(query_params.iter().map(|(k, v)| (*k, *v)))
            .finish();
        
        // Corrected endpoint and protocol based on documentation/testing for TTS
        let endpoint_host = "nls-gateway-ap-southeast-1.aliyuncs.com"; // As per documentation for TTS
        let api_path = "/stream/v1/tts";                               // As per documentation for TTS
        let final_url = format!("https://{}{}?{}", endpoint_host, api_path, query_string); // Use HTTPS

        debug!("Aliyun TTS Request URL: {}", final_url);

        let response = self.http_client
            .get(&final_url)
            .header(ACCEPT, "application/octet-stream") // 修改：使用通用二进制数据类型
            .send()
            .await
            .map_err(|e| {
                error!("Aliyun TTS network request failed: {}", e);
                AiError::RequestError(format!("Aliyun TTS network request failed: {}", e))
            })?;

        let status = response.status();
        
        // 获取响应的 Content-Type 头部，转换为小写以便于比较
        let content_type_header = response.headers().get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();
        
        trace!("Aliyun TTS Response Status: {}, Content-Type: '{}'", status, content_type_header);

        if status.is_success() {
            if content_type_header.contains("audio/mpeg") { // 关键检查点：成功的音频数据响应
                // 响应是音频数据，直接读取字节
                let audio_bytes = response.bytes().await.map_err(|e| {
                    error!("Failed to read Aliyun TTS PCM audio response body: {}", e);
                    AiError::ResponseParseError(format!("Failed to read Aliyun TTS PCM audio response body: {}", e))
                })?;
                
                info!("Aliyun TTS synthesis successful (indicated by Content-Type: audio/mpeg), received {} PCM audio bytes.", audio_bytes.len());
                Ok(audio_bytes.to_vec())
            } else if content_type_header.contains("application/json") || content_type_header.is_empty() {
                // HTTP 200 OK，但 Content-Type 表明这是一个 JSON 错误响应
                let error_body_text = response.text().await.unwrap_or_else(|_| "Failed to read JSON error body".to_string());
                error!(
                    "Aliyun TTS API call returned HTTP 200 OK but Content-Type ('{}') indicates a JSON error. Body: {}",
                    content_type_header, error_body_text
                );
                
                // 尝试解析为 AliyunAsrResponse 或通用错误结构以提取详细信息
                match serde_json::from_str::<AliyunAsrResponse>(&error_body_text) {
                    Ok(parsed_error) => Err(AiError::ApiError {
                        status: status.as_u16(), // HTTP 状态为 200，但业务逻辑错误
                        message: format!(
                            "Aliyun TTS API Error (business logic): Code: {:?}, Message: {:?}, TaskId: {:?}, RequestId: {:?}. Full: {}",
                            parsed_error.code.or_else(|| parsed_error.error_code.clone()),
                            parsed_error.message.or_else(|| parsed_error.error_message.clone()),
                            parsed_error.task_id,
                            parsed_error.request_id,
                            error_body_text
                        ),
                    }),
                    Err(_) => Err(AiError::ApiError { 
                        status: status.as_u16(),
                        message: format!("Aliyun TTS API Error (HTTP 200, Content-Type: {}). Raw Body: {}", content_type_header, error_body_text),
                    }),
                }
            } else {
                // HTTP 200 OK，但 Content-Type 既不是 audio/mpeg 也不是 application/json
                let response_text = response.text().await.unwrap_or_else(|_| "Failed to read unexpected response body".to_string());
                error!(
                    "Aliyun TTS request successful (HTTP {}), but received completely unexpected Content-Type: '{}'. Response body: {}",
                    status, content_type_header, response_text
                );
                
                Err(AiError::ResponseParseError(format!(
                    "Aliyun TTS success with completely unexpected Content-Type: {}. Body: {}",
                    content_type_header, response_text
                )))
            }
        } else {
            // HTTP 状态不是成功（例如，4xx，5xx）
            let error_body_text = response.text().await.unwrap_or_else(|_| "Failed to read error response body for non-200 status".to_string());
            error!("Aliyun TTS API call failed with HTTP Status: {}. Response Body: {}", status, error_body_text);
            
            match serde_json::from_str::<AliyunAsrResponse>(&error_body_text) {
                Ok(parsed_error) => Err(AiError::ApiError {
                    status: status.as_u16(),
                    message: format!(
                        "Aliyun TTS API Error: Code: {:?}, Message: {:?}, TaskId: {:?}, RequestId: {:?}. Full: {}",
                        parsed_error.code.or_else(|| parsed_error.error_code.clone()),
                        parsed_error.message.or_else(|| parsed_error.error_message.clone()),
                        parsed_error.task_id,
                        parsed_error.request_id,
                        error_body_text
                    ),
                }),
                Err(_) => Err(AiError::ApiError {
                    status: status.as_u16(),
                    message: format!("Aliyun TTS API Error (HTTP {}). Raw Body: {}", status, error_body_text),
                }),
            }
        }
    }

    /// Transcribe speech audio to text using Aliyun NLS ASR (一句话识别).
    async fn speech_to_text(&self, audio_data: &[u8], sample_rate_hertz: u32, _language_code: &str) -> Result<String, AiError> {
        debug!("AliyunAuxClient: speech_to_text called. Audio data length: {}", audio_data.len());

         if self.app_key.is_empty() {
            error!("Aliyun AppKey is not configured. STT functionality is unavailable.");
            return Err(AiError::AuthenticationError("Aliyun AppKey not configured for STT.".to_string()));
        }

        let token = self.ensure_valid_token().await.map_err(|e| {
            error!("Failed to ensure valid Aliyun token for STT: {:?}", e);
            e
        })?;
        trace!("Aliyun STT: Using token: {}...", &token[..std::cmp::min(10, token.len())]);


        // 修改: 将临时 String 存储到变量中，延长其生命周期
        let sample_rate_str = sample_rate_hertz.to_string();
        
        let mut query_params = Vec::new();
        query_params.push(("appkey", self.app_key.as_str()));
        query_params.push(("token", token.as_str()));
        query_params.push(("format", "pcm")); // Format is pcm for raw audio bytes
        query_params.push(("sample_rate", sample_rate_str.as_str()));
        query_params.push(("enable_punctuation_prediction", "true"));
        query_params.push(("enable_inverse_text_normalization", "true"));
        // language_code is not directly used in query for Aliyun ASR (一句话识别)
        // as per API docs; it's often project-level or model-inherent.

        let query_string = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(query_params.iter().map(|(k, v)| (*k, *v)))
            .finish();

        let endpoint_host = "nls-gateway-ap-southeast-1.aliyuncs.com"; // CORRECTED HOST
        let api_path = "/stream/v1/asr";                              // Correct path for STT
        // Ensure HTTPS is used for security, unless specifically instructed otherwise for testing.
        // The prompt specified HTTP, so adhering to that for this change.
        let final_url = format!("http://{}{}?{}", endpoint_host, api_path, query_string); 
        
        debug!("Aliyun STT Request URL: {}", final_url);


        let response = self.http_client
            .post(&final_url)
            .header(CONTENT_TYPE, "application/octet-stream") // Sending raw audio bytes
            .header(ACCEPT, "application/json") // Expecting JSON response
            .body(audio_data.to_vec()) // Send audio data as body
            .send()
            .await
            .map_err(|e| {
                error!("Aliyun STT network request failed: {}", e);
                AiError::RequestError(format!("Aliyun STT network request failed: {}", e))
            })?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
            error!("Failed to read Aliyun STT response body: {}", e);
            AiError::ResponseParseError(format!("Failed to read Aliyun STT response body: {}", e))
        })?;
        trace!("Aliyun STT Response Status: {}, Body: {}", status, response_text);

        match serde_json::from_str::<AliyunAsrResponse>(&response_text) {
            Ok(parsed_response) => {
                // Scenario 1: HTTP status is NOT success (e.g., 4xx, 5xx)
                if !status.is_success() {
                    let err_detail_code = parsed_response.code.as_deref()
                        .or(parsed_response.error_code.as_deref())
                        .unwrap_or("N/A");
                    let err_detail_msg = parsed_response.message.as_deref()
                        .or(parsed_response.error_message.as_deref())
                        .unwrap_or(&response_text); // Fallback to full body if no specific message

                    error!(
                        "Aliyun STT API call failed with HTTP Status: {}. API Code: {}, API Message: '{}', TaskId: {:?}, RequestId: {:?}. Full Body: {}",
                        status,
                        err_detail_code,
                        err_detail_msg,
                        parsed_response.task_id,
                        parsed_response.request_id,
                        response_text
                    );
                    return Err(AiError::ApiError {
                        status: status.as_u16(),
                        message: format!(
                            "Aliyun STT API HTTP Error {}. API Code: {}, API Message: '{}'. Full: {}",
                            status,
                            err_detail_code,
                            err_detail_msg,
                            response_text
                        ),
                    });
                }

                // Scenario 2: HTTP status IS success (2xx)
                // Now check the business status from the JSON payload.
                let business_success_by_status_val = parsed_response.status_code_val == Some(20000000);
                let business_success_by_message = parsed_response.message.as_deref().map_or(false, |s| s.eq_ignore_ascii_case("SUCCESS"));
                let business_success_by_status_text = parsed_response.status_text.as_deref().map_or(false, |s| s.eq_ignore_ascii_case("SUCCESS") || s.eq_ignore_ascii_case("TranscriptionCompleted"));

                let business_success = business_success_by_status_val || business_success_by_message || business_success_by_status_text;
                
                if business_success {
                    // 修改: 直接从 parsed_response.result 获取转录文本
                    if let Some(transcribed_text) = parsed_response.result { 
                        if !transcribed_text.is_empty() {
                            info!("Aliyun STT successful. Transcript: {}...", transcribed_text.chars().take(50).collect::<String>());
                            return Ok(transcribed_text);
                        } else {
                            warn!("Aliyun STT successful (API status OK) but transcribed text is empty. TaskId: {:?}, RequestId: {:?}. Body: {}", parsed_response.task_id, parsed_response.request_id, response_text);
                            return Ok(String::new()); 
                        }
                    } else {
                        // Business success indicated, but "result" field is missing.
                        error!("Aliyun STT API reported business success (e.g., status 20000000 or SUCCESS message), but the 'result' field (transcription string) is missing. TaskId: {:?}, RequestId: {:?}. Full Body: {}", parsed_response.task_id, parsed_response.request_id, response_text);
                        return Err(AiError::ResponseParseError(
                            "Aliyun STT success but 'result' field (transcription string) missing.".to_string()
                        ));
                    }
                } else {
                    // HTTP 200 OK, but business logic status in JSON indicates failure.
                    let api_status_val_str = parsed_response.status_code_val.map_or_else(|| "N/A".to_string(), |v| v.to_string());
                    let api_status_text_str = parsed_response.status_text.as_deref().unwrap_or("N/A");
                    let api_code_str = parsed_response.code.as_deref().or(parsed_response.error_code.as_deref()).unwrap_or("N/A");
                    let api_message_str = parsed_response.message.as_deref().or(parsed_response.error_message.as_deref()).unwrap_or("No specific error message from API.");

                    error!(
                        "Aliyun STT API call returned HTTP 200 OK, but business logic failed. API Numeric Status: {}, API StatusText: '{}', API Code: {}, API Message: '{}'. TaskId: {:?}, RequestId: {:?}. Full Body: {}",
                        api_status_val_str,
                        api_status_text_str,
                        api_code_str,
                        api_message_str,
                        parsed_response.task_id,
                        parsed_response.request_id,
                        response_text
                    );
                    return Err(AiError::ApiError {
                        status: status.as_u16(), // HTTP status was 200
                        message: format!("Aliyun STT business logic error. API Status: {}, API StatusText: '{}', API Message: '{}'", api_status_val_str, api_status_text_str, api_message_str),
                    });
                }
            }
            Err(parse_err) => {
                // JSON parsing failed.
                error!(
                    "Failed to parse Aliyun STT response JSON. HTTP Status: {}. Parse Error: {}. Body: {}",
                    status, parse_err, response_text
                );
                if status.is_success() {
                    return Err(AiError::ResponseParseError(format!(
                        "Failed to parse successful Aliyun STT JSON response (HTTP {}): {}. Body: {}",
                        status, parse_err, response_text
                    )));
                } else {
                    // HTTP error and failed to parse body for more details.
                     return Err(AiError::ApiError {
                        status: status.as_u16(),
                        message: format!("Aliyun STT API HTTP Error {} and failed to parse error response body. Raw Body: {}", status, response_text),
                    });
                }
            }
        }
    }
} 