import { useWebsocketStore } from '../store/websocketStore';
import { useLogStore } from '../store/logStore';

// --- 修改 URL 定义 ---
// 动态地从前端服务的主机名确定后端主机名
const backendHostname = window.location.hostname;
const backendPort = 5900; // 保留后端端口
const API_BASE_URL = `http://${backendHostname}:${backendPort}/api`; // Construct URL dynamically (Used by fetchApi if added later)
const WEBSOCKET_URL = `ws://${backendHostname}:${backendPort}/ws`;
// --- URL 定义修改结束 ---

let socket: WebSocket | null = null;

export function initializeWebSocket() {
    if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {
        console.warn('WebSocket 已经处于打开或正在连接状态。');
        return;
    }

    console.log(`尝试连接 WebSocket: ${WEBSOCKET_URL}`); // 现在会显示动态的 URL
    useWebsocketStore.getState().setStatus('Connecting'); // 通过 Zustand 更新状态

    try {
        socket = new WebSocket(WEBSOCKET_URL);

        socket.onopen = () => {
            console.log('WebSocket 连接已建立。');
            useWebsocketStore.getState().setStatus('Connected');
        };

        socket.onclose = (event) => {
            console.warn(`WebSocket 连接已关闭: ${event.code} ${event.reason}`);
            useWebsocketStore.getState().setStatus('Disconnected');
            socket = null; // 清除 socket 实例
            // 可选：在此处实现重连逻辑
        };

        socket.onerror = (error) => {
            console.error('WebSocket 错误:', error);
            useWebsocketStore.getState().setStatus('Error');
            // 考虑在这里关闭 socket（如果尚未关闭）
            socket?.close();
            socket = null;
        };

        socket.onmessage = (event) => {
            try {
                const messageData = JSON.parse(event.data);
                // Log raw message for debugging, can be removed later
                // console.debug('WebSocket message received (raw):', event.data);
                // console.debug('WebSocket message received (parsed):', messageData);

                // --- Add Log Message Handling ---
                // Check if the message type indicates a log message from the backend
                // (Adjust 'log_message' and 'message' field based on actual backend event structure)
                // Assuming backend sends logs as simple strings for now based on elfradio_api code
                if (typeof event.data === 'string') {
                    // Add the log string directly to the Zustand store
                    // Use the message content directly if it's just a string
                    useLogStore.getState().addMessage(event.data);
                } else if (messageData && messageData.type === 'log_message' && typeof messageData.message === 'string') {
                    // Or handle structured log messages if backend sends them
                    useLogStore.getState().addMessage(messageData.message);
                }
                // --- End Log Message Handling ---

                // TODO: Handle other structured message types later (e.g., status updates, task results)

            } catch (e) {
                console.error('Failed to parse WebSocket message:', event.data, e);
                // Add raw data to log store on parse failure if it's a string
                if (typeof event.data === 'string') {
                   useLogStore.getState().addMessage(`[RAW/ERROR] ${event.data}`);
                } else {
                   useLogStore.getState().addMessage(`[RAW/ERROR] Received non-string, non-JSON message.`);
                }
            }
        };

    } catch (error) {
        console.error('创建 WebSocket 失败:', error);
        useWebsocketStore.getState().setStatus('Error');
    }
}

// 可选：添加一个显式关闭连接的函数
export function disconnectWebSocket() {
    if (socket) {
        console.log('关闭 WebSocket 连接。');
        socket.close();
        socket = null;
        useWebsocketStore.getState().setStatus('Disconnected');
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
