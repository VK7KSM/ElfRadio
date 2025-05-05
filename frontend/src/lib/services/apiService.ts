import { useAuthStore } from '../store/authStore'; // Placeholder for future auth

// --- Configuration ---
// --- Modify URL Construction ---
// Determine hostname dynamically
const backendHostname = window.location.hostname;
const backendPort = 5900; // Keep the backend port
const API_BASE_URL = `http://${backendHostname}:${backendPort}/api`; // Construct URL dynamically
// --- End URL Modification ---

// --- Frontend TaskMode type (matching backend enum) ---
export type TaskMode = 
  | 'GeneralCommunication'
  | 'AirbandListening'
  | 'SatelliteCommunication'
  | 'EmergencyCommunication'
  | 'MeshtasticGateway'
  | 'SimulatedQsoPractice';

// --- Helper for making API requests ---
interface RequestOptions extends RequestInit {
  useAuth?: boolean; // Flag to include auth token
}

async function fetchApi<T>(endpoint: string, options: RequestOptions = {}): Promise<T> {
    const url = `${API_BASE_URL}${endpoint}`; // Construct full URL
    const headers: HeadersInit = {
        'Content-Type': 'application/json',
        'Accept': 'application/json',
        ...options.headers, // Allow overriding headers
    };

    // Placeholder for adding Authentication header later
    if (options.useAuth) {
         const token = useAuthStore.getState().token; // Example: Get token from auth store
         if (token) {
             (headers as Record<string, string>)['Authorization'] = `Bearer ${token}`; // Type assertion for header access
         } else {
              console.warn('Attempted to make authenticated API call without token.');
              // Handle missing token case? Throw error?
         }
    }

    try {
        const response = await fetch(url, {
            ...options, // Spread other options like method, body
            headers,
        });

        if (!response.ok) {
            // 修复 "body stream already read" 错误的处理方法
            let errorText = `HTTP error ${response.status}`;
            let errorData: any = null;
            
            try {
                // 只读取响应体一次
                const bodyText = await response.text();
                try {
                    // 尝试解析为 JSON
                    errorData = JSON.parse(bodyText);
                    errorText = errorData?.error || bodyText;
                } catch (e) {
                    // 如果解析失败，使用原始文本
                    errorText = bodyText;
                }
            } catch (e) {
                // 如果无法读取响应体
                console.error("Failed to read error response body", e);
            }
            
            console.error(`API Error ${response.status}: ${response.statusText}`, errorData || errorText);
            throw new Error(`API request to ${endpoint} failed with status ${response.status}: ${errorText}`);
        }

        // 处理无内容响应 (204 No Content)
        if (response.status === 204) {
            return null as T;
        }

        // 默认处理为 JSON 响应
        return await response.json() as T;
    } catch (error) {
        console.error(`Network or fetch error for ${url}:`, error);
        throw error; // 重新抛出错误由调用者处理
    }
}

// --- Specific API Functions ---

/**
 * Checks the health of the backend API server.
 * @returns Promise<string> - Typically returns "OK" on success.
 */
export async function checkBackendHealth(): Promise<string> {
    // The health endpoint might return plain text "OK"
    const url = `${API_BASE_URL}/health`; // This now uses the dynamic base URL
    try {
         const response = await fetch(url);
         if (!response.ok) {
             throw new Error(`Backend health check failed: ${response.status}`);
         }
         const text = await response.text();
         // Add check for the actual content if necessary
         if (text.trim() !== "OK") {
            console.warn(`Backend health check returned unexpected text: ${text}`);
            // Decide whether to throw an error or return the text
            // throw new Error(`Backend health check returned unexpected text: ${text}`);
         }
         return text; // Expecting plain text "OK"
    } catch (error) {
         console.error("Backend health check failed:", error);
         throw error;
    }
   // Or using fetchApi if health returns JSON like {"status": "OK"}
   // return fetchApi<{ status: string }>('/health').then(res => res.status);
}

// --- Task Control Functions ---

// Define TaskInfo type based on backend response
interface StartTaskResponse {
  task_id: string; // Expecting UUID as string
}

/**
 * Start a task with the specified mode.
 * @param mode The task mode to start
 * @returns Promise with the task ID on success
 */
export async function startTask(mode: TaskMode): Promise<StartTaskResponse> {
  console.info("API Call: Starting task with mode:", mode);
  try {
    const response = await fetchApi<StartTaskResponse>('/start_task', {
      method: 'POST',
      body: JSON.stringify({ mode }), // Send mode in request body
    });
    console.info("API Response: Task started successfully:", response);
    return response;
  } catch (error) {
    console.error("API Error: Failed to start task:", error);
    throw error; // Re-throw for caller to handle
  }
}

/**
 * Stop the currently running task.
 * @returns Promise that resolves when task is successfully stopped
 */
export async function stopTask(): Promise<void> {
  console.info("API Call: Stopping current task...");
  try {
    // Use fetchApi, but expect no JSON response content on success (status 200/204)
    const url = `${API_BASE_URL}/stop_task`;
    const response = await fetch(url, { 
      method: 'POST', 
      headers: {'Accept': '*/*'} 
    });

    if (!response.ok) {
      let errorData;
      try { 
        errorData = await response.json(); 
      } catch (e) { 
        /* ignore parsing errors */ 
      }
      console.error(`API Error ${response.status}: ${response.statusText}`, errorData || await response.text());
      throw new Error(`API request failed with status ${response.status}: ${errorData?.error || response.statusText}`);
    }
    console.info("API Response: Stop task request successful.");
    // No return value needed for void Promise
  } catch (error) {
    console.error("API Error: Failed to stop task:", error);
    throw error;
  }
}

// --- Communication Functions ---

/**
 * Sends a text message to the backend for transmission.
 * @param text The text message to send.
 * @returns Promise<void> Resolves on success (HTTP 202 Accepted), rejects on failure.
 */
export async function sendTextMessage(text: string): Promise<void> {
  console.log('Sending text message:', text);
  const url = `${API_BASE_URL}/send_text`;
  const body = JSON.stringify({ text });

  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: body,
    });

    // 修改错误处理逻辑
    if (!response.ok) {
      let errorText = `HTTP error ${response.status}`;
      let errorData: any = null;
      
      try {
        // 只读取响应体一次
        const bodyText = await response.text();
        try {
          // 尝试解析为 JSON
          errorData = JSON.parse(bodyText);
          errorText = errorData?.error || bodyText;
        } catch (e) {
          // 如果解析失败，使用原始文本
          errorText = bodyText;
        }
      } catch (e) {
        // 如果无法读取响应体
        console.error("Failed to read error response body", e);
      }
      
      console.error('sendTextMessage failed:', errorText, errorData);
      throw new Error(errorText);
    }
    
    console.log('sendTextMessage successful');
    return; // 成功返回
  } catch (error) {
    console.error('Network or fetch error in sendTextMessage:', error);
    // 确保错误被重新抛出为 Error 对象
    if (error instanceof Error) {
      throw error;
    } else {
      throw new Error(String(error));
    }
  }
}

console.log('apiService.ts loaded'); // Log to confirm loading 