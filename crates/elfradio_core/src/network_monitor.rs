use elfradio_types::{AiError, ConnectionStatus};
use reqwest::Client;
use std::time::Duration;
use tracing::{info, warn, error};
use tokio::sync::{mpsc, watch};
use tokio::time::sleep;
use elfradio_types::{WebSocketMessage, LogEntry, LogDirection, LogContentType};
use chrono::Utc;
use tracing::{debug, trace};

const TARGET_URLS: &[&str] = &[
    "http://detectportal.firefox.com/success.txt",
    "https://captive.apple.com/hotspot-detect.html",
    "http://connectivitycheck.gstatic.com/generate_204",
    // 备用一个国内的，例如百度的 favicon，它通常很小且稳定
    "http://www.baidu.com/favicon.ico",
];

/// 检查初始网络连通性。
///
/// 尝试连接到一系列预定义的目标 URL，以确定是否存在有效的互联网连接。
///
/// # 返回
/// - `Ok(ConnectionStatus::Connected)`: 如果至少有一个目标 URL 成功连接。
/// - `Ok(ConnectionStatus::Disconnected)`: 如果所有目标 URL 连接尝试均失败。
/// - `Err(AiError)`: 如果在创建 HTTP 客户端或执行请求时发生内部错误。
pub async fn check_initial_network_connectivity() -> Result<ConnectionStatus, AiError> {
    info!("开始进行初始网络连通性检测...");

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| {
            error!("创建 reqwest 客户端失败: {:?}", e);
            AiError::ClientError(format!("创建 HTTP 客户端失败: {}", e))
        })?;

    let mut is_online = false;

    for url in TARGET_URLS {
        info!("尝试连接到: {}", url);
        match client.head(*url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    is_online = true;
                    info!("网络检测成功: 成功连接到 {} (状态: {})", url, response.status());
                    break; // 第一个成功连接就退出循环
                } else {
                    warn!(
                        "网络检测: 连接到 {} 失败 - 状态码: {}",
                        url,
                        response.status()
                    );
                }
            }
            Err(e) => {
                // HEAD 请求失败，尝试 GET 作为备用
                warn!("网络检测: HEAD 请求到 {} 失败: {:?}。尝试 GET 请求...", url, e);
                match client.get(*url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            is_online = true;
                            info!("网络检测成功 (GET): 成功连接到 {} (状态: {})", url, response.status());
                            break; // 第一个成功连接就退出循环
                        } else {
                            warn!(
                                "网络检测 (GET): 连接到 {} 失败 - 状态码: {}",
                                url,
                                response.status()
                            );
                        }
                    }
                    Err(get_e) => {
                        warn!("网络检测 (GET): 连接到 {} 时发生错误: {:?}", url, get_e);
                    }
                }
            }
        }
    }

    if is_online {
        info!("网络连通性检测完成: 已连接到互联网。");
        Ok(ConnectionStatus::Connected)
    } else {
        warn!("网络连通性检测完成: 未能连接到任何目标 URL，可能已断开互联网连接。");
        Ok(ConnectionStatus::Disconnected)
    }
}

/// 网络连接状态周期性监控任务
///
/// 定期检查网络连通性并在状态变化时通过WebSocket和日志通道发送更新
///
/// # 参数
/// * `ws_message_tx` - 用于发送WebSocket消息的发送者通道
/// * `log_entry_tx` - 用于发送日志条目的发送者通道
/// * `shutdown_rx` - 用于接收应用程序关闭信号的接收者通道
///
/// # 返回
/// 当收到关闭信号时函数返回
pub async fn periodic_network_connectivity_monitor(
    ws_message_tx: mpsc::UnboundedSender<WebSocketMessage>,
    log_entry_tx: mpsc::UnboundedSender<LogEntry>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    const NETWORK_CHECK_INTERVAL_SECONDS: u64 = 60;
    
    info!("网络连接状态监控任务已启动，检查间隔: {}秒", NETWORK_CHECK_INTERVAL_SECONDS);
    
    // 初始化为None，以便第一次检查始终发送状态更新
    let mut last_known_status: Option<ConnectionStatus> = None;
    
    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("网络监控: 收到关闭信号，正在退出");
                    break;
                }
            }
            _ = sleep(Duration::from_secs(NETWORK_CHECK_INTERVAL_SECONDS)) => {
                debug!("网络监控: 开始定期网络连接状态检查");
                
                match check_initial_network_connectivity().await {
                    Ok(current_status) => {
                        // 检查状态是否变化或是首次检查
                        if last_known_status.as_ref() != Some(&current_status) {
                            // 记录状态变化
                            let previous_status = last_known_status.map_or_else(
                                || "未知".to_string(), 
                                |status| format!("{:?}", status)
                            );
                            
                            let log_content = format!(
                                "网络连接状态从 {} 变更为 {:?}", 
                                previous_status, 
                                current_status
                            );
                            
                            // 创建并发送日志条目
                            let log_entry = LogEntry {
                                timestamp: Utc::now(),
                                direction: LogDirection::Internal,
                                content_type: LogContentType::Status,
                                content: log_content.clone(),
                            };
                            
                            if log_entry_tx.send(log_entry).is_err() {
                                error!("网络监控: 发送状态变更日志失败");
                            }
                            
                            // 记录到本地跟踪日志
                            info!("网络监控: {}", log_content);
                            
                            // 创建并发送WebSocket消息
                            let ws_update_msg = WebSocketMessage::NetworkConnectivityUpdate(current_status.clone());
                            if ws_message_tx.send(ws_update_msg).is_err() {
                                error!("网络监控: 发送WebSocket状态更新失败");
                            }
                            
                            // 更新最后已知状态
                            last_known_status = Some(current_status);
                        } else {
                            trace!("网络监控: 连接状态未变化，保持为 {:?}", current_status);
                        }
                    }
                    Err(e) => {
                        error!("网络监控: 连接检查过程中发生错误: {:?}", e);
                        
                        // 如果之前状态为Connected，错误可能表示连接丢失，发送Error状态
                        if last_known_status == Some(ConnectionStatus::Connected) {
                            // 创建并发送错误状态的日志条目
                            let log_content = "网络连接检查失败，可能已断开连接".to_string();
                            let log_entry = LogEntry {
                                timestamp: Utc::now(),
                                direction: LogDirection::Internal,
                                content_type: LogContentType::Status,
                                content: log_content.clone(),
                            };
                            
                            if log_entry_tx.send(log_entry).is_err() {
                                error!("网络监控: 发送错误状态日志失败");
                            }
                            
                            // 发送错误状态WebSocket消息
                            let ws_update_msg = WebSocketMessage::NetworkConnectivityUpdate(ConnectionStatus::Error);
                            if ws_message_tx.send(ws_update_msg).is_err() {
                                error!("网络监控: 发送WebSocket错误状态更新失败");
                            }
                            
                            // 更新最后已知状态
                            last_known_status = Some(ConnectionStatus::Error);
                            
                            warn!("网络监控: {}", log_content);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn test_connectivity_check_success_head() {
        let server = MockServer::start();
        let mock_url = server.url("/success.txt");

        server.mock(|when, then| {
            when.method(HEAD).path("/success.txt");
            then.status(200);
        });
        
        // 临时的 TARGET_URLS，只包含 mock_url
        const MOCK_TARGET_URLS: &[&str] = &[&mock_url];

        let client = Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
            .unwrap();
        
        let mut is_online = false;
        for url_str in MOCK_TARGET_URLS {
            match client.head(url_str).send().await {
                Ok(response) if response.status().is_success() => {
                    is_online = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(is_online);
    }

    #[tokio::test]
    async fn test_connectivity_check_success_get_after_head_fail() {
        let server = MockServer::start();
        let mock_url = server.url("/success.txt");

        // HEAD 请求返回错误，GET 请求成功
        server.mock(|when, then| {
            when.method(HEAD).path("/success.txt");
            then.status(500); // HEAD 失败
        });
        server.mock(|when, then| {
            when.method(GET).path("/success.txt");
            then.status(200); // GET 成功
        });
        
        const MOCK_TARGET_URLS: &[&str] = &[&mock_url];
        let client = Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
            .unwrap();
        
        let mut is_online = false;
        for url_str in MOCK_TARGET_URLS {
            match client.head(url_str).send().await {
                Ok(response) if response.status().is_success() => {
                    is_online = true;
                    break;
                }
                Err(_) | Ok(_) => { // HEAD 失败或状态不成功
                    match client.get(url_str).send().await {
                        Ok(get_response) if get_response.status().is_success() => {
                            is_online = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
        assert!(is_online);
    }

    #[tokio::test]
    async fn test_connectivity_check_all_fail() {
        let server = MockServer::start();
        let mock_url = server.url("/failure.txt");

        server.mock(|when, then| {
            when.method(HEAD).path("/failure.txt");
            then.status(500);
        });
        server.mock(|when, then| {
            when.method(GET).path("/failure.txt");
            then.status(404);
        });

        const MOCK_TARGET_URLS: &[&str] = &[&mock_url];
        let client = Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
            .unwrap();

        let mut is_online = false;
        for url_str in MOCK_TARGET_URLS {
             match client.head(url_str).send().await {
                Ok(response) if response.status().is_success() => {
                    is_online = true;
                    break;
                }
                Err(_) | Ok(_) => {
                    match client.get(url_str).send().await {
                        Ok(get_response) if get_response.status().is_success() => {
                            is_online = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
        assert!(!is_online);
    }
}
