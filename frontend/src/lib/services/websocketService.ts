import { useWebsocketStore } from '../store/websocketStore';
import { useLogStore } from '../store/logStore';
import { useSystemStatusStore, ConnectionStatus, SystemServiceStatus } from '../store/systemStatusStore';
import { useBackendStatusStore } from '../store/backendStatusStore';
import { checkBackendHealth } from './apiService'; // 

// --- 修改 URL 定义 ---
// 动态地从前端服务的主机名确定后端主机名
const backendHostname = window.location.hostname;
const backendPort = 5900; // 保留后端端口
// const API_BASE_URL = `http://${backendHostname}:${backendPort}/api`; // Construct URL dynamically (Used by fetchApi if added later) - 已移除
const WEBSOCKET_URL = `ws://${backendHostname}:${backendPort}/ws`;
// --- URL 定义修改结束 ---

let socket: WebSocket | null = null;
const RECONNECTION_DELAY_MS = 1500; // Changed to 1.5 seconds
let reconnectionAttempt = 0; 
let reconnectionTimeoutId: number | null = null; // Changed NodeJS.Timeout to number

// Interface for the structured WebSocket messages from the backend
interface BackendWebSocketMessage {
  type: string; // e.g., "Log", "RadioStatusUpdate", "LlmStatusUpdate", etc.
  payload: any; // The content of the message
}

export function initializeWebSocket() {
    if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {
        console.warn('WebSocket 已经处于打开或正在连接状态。');
        return;
    }

    console.log(`尝试连接 WebSocket: ${WEBSOCKET_URL}`); 
    useWebsocketStore.getState().setStatus('Connecting'); 

    try {
        socket = new WebSocket(WEBSOCKET_URL);

        socket.onopen = () => {
            console.log('WebSocket 连接已建立。');
            useWebsocketStore.getState().setStatus('Connected');
            
            if (reconnectionTimeoutId) {
                clearTimeout(reconnectionTimeoutId);
                reconnectionTimeoutId = null;
                console.log('[WebSocketService] Connection established, cleared pending reconnection timeout.');
            }
            reconnectionAttempt = 0;
            console.log('[WebSocketService] Reconnection attempt counter reset.');

            // --- BEGIN ADDED HEALTH CHECK ON RECONNECT ---
            console.log('[WebSocketService] WebSocket connected/reconnected. Performing backend health check...');
            const setBackendStatus = useBackendStatusStore.getState().setStatus;
            setBackendStatus('Checking'); // Set to checking while request is in flight
            checkBackendHealth()
                .then(result => {
                    // The checkBackendHealth in apiService is expected to return the raw text 'OK' or throw an error.
                    // It does not return an object with a trim method, usually.
                    // Let's adjust the condition based on how typical fetch responses are handled,
                    // assuming checkBackendHealth resolves with 'OK' text on success.
                    if (result === 'OK') { // Simplified check, assuming result is already trimmed or exact 'OK'
                        setBackendStatus('OK');
                        console.log('[WebSocketService] Backend health check after connect: OK');
                    } else {
                        setBackendStatus('Error');
                        console.error('[WebSocketService] Backend health check after connect failed: Unexpected response:', result);
                    }
                })
                .catch(error => {
                    setBackendStatus('Error');
                    console.error('[WebSocketService] Backend health check after connect failed:', error);
                });
            // --- END ADDED HEALTH CHECK ON RECONNECT ---
        };

        socket.onclose = (event) => {
            console.warn(`WebSocket 连接已关闭: ${event.code} ${event.reason}`);
            useWebsocketStore.getState().setStatus('Disconnected');
            socket = null; 

            // Status Reset Logic (as implemented in previous step)
            console.log('[WebSocketService] Connection closed. Resetting backend-dependent statuses.');
            const systemStatusSetters = useSystemStatusStore.getState();
            systemStatusSetters.setNetworkStatus('Disconnected');
            systemStatusSetters.setRadioStatus('Unknown'); 
            systemStatusSetters.setSdrStatus('Disconnected');
            systemStatusSetters.setLlmStatus('Unknown');     
            systemStatusSetters.setSttStatus('Unknown');     
            systemStatusSetters.setTtsStatus('Unknown');     
            systemStatusSetters.setTranslateStatus('Unknown'); 
            useBackendStatusStore.getState().setStatus('Error');
            
            // --- BEGIN MODIFIED AUTO-RECONNECT LOGIC ---
            if (reconnectionTimeoutId) {
                clearTimeout(reconnectionTimeoutId);
                // reconnectionTimeoutId = null; // Clearing here might be premature if setTimeout is called immediately after
            }
    
            reconnectionAttempt++; 
            console.log(`[WebSocketService] Scheduling reconnection attempt ${reconnectionAttempt} in ${RECONNECTION_DELAY_MS / 1000} seconds...`);
            
            reconnectionTimeoutId = setTimeout(() => {
                console.log(`[WebSocketService] Attempting reconnection (attempt ${reconnectionAttempt})...`);
                initializeWebSocket(); 
                reconnectionTimeoutId = null; 
            }, RECONNECTION_DELAY_MS);
            // --- END MODIFIED AUTO-RECONNECT LOGIC ---
        };

        socket.onerror = (error) => {
            console.error('WebSocket 错误:', error);
            useWebsocketStore.getState().setStatus('Error');
            // socket?.close(); // The onclose handler will be triggered by the browser if the error causes a close.
                           // If it doesn't close, we might not want to force it here without onclose logic running.
                           // For now, let onclose handle the reconnection attempt after an error-induced close.
            
            // Status Reset Logic (as implemented in previous step)
            console.log('[WebSocketService] Connection error. Resetting backend-dependent statuses.');
            const systemStatusSetters = useSystemStatusStore.getState();
            systemStatusSetters.setNetworkStatus('Error'); 
            systemStatusSetters.setRadioStatus('Error');   
            systemStatusSetters.setSdrStatus('Error'); 
            systemStatusSetters.setLlmStatus('Error');
            systemStatusSetters.setSttStatus('Error');
            systemStatusSetters.setTtsStatus('Error');
            systemStatusSetters.setTranslateStatus('Error');
            useBackendStatusStore.getState().setStatus('Error');

            // Note: If onerror does not always trigger onclose, reconnection logic might need to be
            // duplicated or called from here as well. Standard browser behavior is that 'close'
            // will eventually fire after an 'error' that terminates the connection.
            // We will rely on onclose for scheduling reconnections.
            // If socket is still open after an error, manually closing it here might be an option
            // to ensure onclose fires for reconnection.
             if (socket && socket.readyState !== WebSocket.CLOSING && socket.readyState !== WebSocket.CLOSED) {
                 socket.close(); // Ensure onclose is triggered if error didn't close it.
             } else if (!socket) { // If socket is already null (e.g. by onclose)
                 // This case implies onclose might have already run or is about to.
                 // To be safe and ensure a retry if onclose isn't triggered by this specific error scenario:
                console.log('[WebSocketService] onerror detected socket is null or already closing/closed. Ensuring reconnection is scheduled if not already.');
                if (!reconnectionTimeoutId) { // Only schedule if not already scheduled by a concurrent onclose
                    reconnectionAttempt++;
                    console.log(`[WebSocketService] onerror: Scheduling reconnection attempt ${reconnectionAttempt} in ${RECONNECTION_DELAY_MS / 1000} seconds due to error and no active reconnection.`);
                    reconnectionTimeoutId = setTimeout(() => {
                        console.log(`[WebSocketService] onerror: Attempting reconnection (attempt ${reconnectionAttempt})...`);
                        initializeWebSocket();
                        reconnectionTimeoutId = null;
                    }, RECONNECTION_DELAY_MS);
                }
             }
        };

        socket.onmessage = (event) => {
            // --- BEGIN ADDED TOP-LEVEL DEBUG LOG ---
            console.log('[WebSocketService RAW MSG <<]', event.data);
            // --- END ADDED TOP-LEVEL DEBUG LOG ---
    
            try {
                const rawData = event.data;
                // First, attempt to parse as JSON, assuming most messages will be structured
                let messageData: BackendWebSocketMessage | null = null;
                let isJson = false;
                if (typeof rawData === 'string') {
                    try {
                        messageData = JSON.parse(rawData) as BackendWebSocketMessage;
                        isJson = true;
                    } catch (e) {
                        // Not a JSON string, might be a simple log string from older backend logic
                        console.warn('Received WebSocket message that is not valid JSON:', rawData);
                    }
                } else {
                    console.warn('Received non-string WebSocket message:', rawData);
                    useLogStore.getState().addMessage(`[RAW/UNEXPECTED_TYPE] Received non-string WebSocket message.`);
                    return; // Exit if not a string, as we expect stringified JSON or plain strings
                }
    
                if (isJson && messageData && typeof messageData.type === 'string') {
                    // --- BEGIN ADDED PARSED_DATA DEBUG LOG ---
                    console.log('[WebSocketService PARSED MSG]', JSON.parse(JSON.stringify(messageData))); // Deep clone for clean logging
                    // --- END ADDED PARSED_DATA DEBUG LOG ---

                    const systemStatusStore = useSystemStatusStore.getState(); // Get store once
    
                    switch (messageData.type) {
                        case 'Log': // Assuming 'Log' is the type for LogEntry messages
                            if (messageData.payload && typeof messageData.payload.content === 'string') {
                                useLogStore.getState().addMessage(messageData.payload.content);
                            } else {
                                 useLogStore.getState().addMessage(`[Log] Received log message with unexpected payload structure.`);
                            }
                            break;
                        
                        // Handle new system status updates
                        case 'UserUuidUpdate':
                            systemStatusStore.setUserUuid(messageData.payload as string | null);
                            console.log('SystemStatusStore: UserUUID updated via WebSocket:', messageData.payload);
                            break;
                        case 'RadioStatusUpdate':
                            systemStatusStore.setRadioStatus(messageData.payload as ConnectionStatus);
                            console.log('SystemStatusStore: RadioStatus updated via WebSocket:', messageData.payload);
                            break;
                        case 'SdrStatusUpdate':
                            systemStatusStore.setSdrStatus(messageData.payload as ConnectionStatus);
                            console.log('SystemStatusStore: SdrStatus updated via WebSocket:', messageData.payload);
                            break;
                        case 'LlmStatusUpdate':
                            systemStatusStore.setLlmStatus(messageData.payload as SystemServiceStatus);
                            console.log('SystemStatusStore: LlmStatus updated via WebSocket:', messageData.payload);
                            break;
                        case 'SttStatusUpdate':
                            systemStatusStore.setSttStatus(messageData.payload as SystemServiceStatus);
                            console.log('SystemStatusStore: SttStatus updated via WebSocket:', messageData.payload);
                            break;
                        case 'TtsStatusUpdate':
                            systemStatusStore.setTtsStatus(messageData.payload as SystemServiceStatus);
                            console.log('SystemStatusStore: TtsStatus updated via WebSocket:', messageData.payload);
                            break;
                        case 'TranslateStatusUpdate':
                            systemStatusStore.setTranslateStatus(messageData.payload as SystemServiceStatus);
                            console.log('SystemStatusStore: TranslateStatus updated via WebSocket:', messageData.payload);
                            break;
                        case 'NetworkConnectivityUpdate':
                            const receivedNetworkPayload = messageData.payload as ConnectionStatus; 

                            console.log(
                                '[WebSocketService DEBUG] Received NetworkConnectivityUpdate. Raw Payload:', 
                                messageData.payload, 
                                '| Typeof Payload:', typeof messageData.payload
                            );
                            
                            const storeStateBeforeUpdate = useSystemStatusStore.getState();
                            console.log(
                                '[WebSocketService DEBUG] systemStatusStore.networkStatus BEFORE update:', 
                                storeStateBeforeUpdate.networkStatus
                            );

                            if (['Connected', 'Disconnected', 'Checking', 'Error', 'Unknown'].includes(receivedNetworkPayload)) {
                                useSystemStatusStore.getState().setNetworkStatus(receivedNetworkPayload);
                                
                                const storeStateAfterUpdate = useSystemStatusStore.getState();
                                console.log(
                                    '[WebSocketService DEBUG] systemStatusStore.setNetworkStatus called with:', 
                                    receivedNetworkPayload
                                );
                                console.log(
                                    '[WebSocketService DEBUG] systemStatusStore.networkStatus AFTER update:', 
                                    storeStateAfterUpdate.networkStatus
                                );
                            } else {
                                console.error(
                                    '[WebSocketService ERROR] Received NetworkConnectivityUpdate with INVALID payload:', 
                                    messageData.payload,
                                    '. Status not updated.'
                                );
                            }
                            break;
                        
                        default:
                            console.warn('Received unknown structured WebSocket message type:', messageData.type);
                            useLogStore.getState().addMessage(`[RAW_JSON_UNKNOWN_TYPE] ${JSON.stringify(messageData)}`);
                            break;
                    }
                } else if (typeof rawData === 'string') {
                    useLogStore.getState().addMessage(rawData);
                }
    
            } catch (e: any) { 
                console.error('[WebSocketService FATAL] Error processing WebSocket message. Raw data:', event.data, 'Error Object:', e, 'Error Message:', e?.message, 'Error Stack:', e?.stack);
                if (typeof event.data === 'string') {
                   useLogStore.getState().addMessage(`[RAW/FATAL_PROCESSING_ERROR] ${event.data}`);
                } else {
                   useLogStore.getState().addMessage(`[RAW/FATAL_PROCESSING_ERROR] Received non-string message that caused a fatal processing error.`);
                }
            }
        };

    } catch (error) {
        console.error('创建 WebSocket 失败:', error);
        useWebsocketStore.getState().setStatus('Error');
        // If WebSocket constructor itself fails, we should also attempt a reconnect.
        // This scenario won't trigger onclose or onerror of the socket instance.
        if (!reconnectionTimeoutId) { // Only schedule if not already scheduled
            reconnectionAttempt++;
            console.log(`[WebSocketService] WebSocket constructor failed. Scheduling reconnection attempt ${reconnectionAttempt} in ${RECONNECTION_DELAY_MS / 1000} seconds...`);
            reconnectionTimeoutId = setTimeout(() => {
                console.log(`[WebSocketService] Constructor failure: Attempting reconnection (attempt ${reconnectionAttempt})...`);
                initializeWebSocket();
                reconnectionTimeoutId = null;
            }, RECONNECTION_DELAY_MS);
        }
    }
}

export function disconnectWebSocket() {
    if (reconnectionTimeoutId) {
        clearTimeout(reconnectionTimeoutId);
        reconnectionTimeoutId = null;
        console.log('[WebSocketService] Cleared pending reconnection on manual disconnect.');
    }
    if (socket) {
        console.log('手动关闭 WebSocket 连接。');
        // Prevent onclose and onerror from triggering auto-reconnect during manual disconnect
        socket.onclose = null; 
        socket.onerror = null;
        socket.close();
        socket = null;
        useWebsocketStore.getState().setStatus('Disconnected');
        // Optionally, reset system statuses here too, or rely on UI to reflect WS disconnected.
        // For consistency, let's reset them like in the automatic onclose.
        console.log('[WebSocketService] Manual disconnect. Resetting backend-dependent statuses.');
        const systemStatusSetters = useSystemStatusStore.getState();
        systemStatusSetters.setNetworkStatus('Disconnected');
        systemStatusSetters.setRadioStatus('Unknown');
        systemStatusSetters.setSdrStatus('Disconnected');
        systemStatusSetters.setLlmStatus('Unknown');
        systemStatusSetters.setSttStatus('Unknown');
        systemStatusSetters.setTtsStatus('Unknown');
        systemStatusSetters.setTranslateStatus('Unknown');
        useBackendStatusStore.getState().setStatus('Error'); // Or 'Unknown'
    }
}

// 可选：添加一个发送消息的函数（如果以后需要）
// export function sendWebSocketMessage(message: any) {
//   if (socket && socket.readyState === WebSocket.OPEN) {
//     socket.send(JSON.stringify(message));
//   } else {
//     console.error('WebSocket 未连接，无法发送消息。');
//   }
// }
