import { create } from 'zustand';

type WebSocketStatus = 'Connecting' | 'Connected' | 'Disconnected' | 'Error';

interface WebSocketState {
  status: WebSocketStatus;
  setStatus: (status: WebSocketStatus) => void;
}

// Create the Zustand store
export const useWebsocketStore = create<WebSocketState>((set) => ({
  status: 'Connecting', // Initial status
  setStatus: (newStatus) => set({ status: newStatus }),
}));

// Export a hook for convenience (optional but common)
export const useWebsocketStatus = () => useWebsocketStore((state) => state.status);

// Your websocket.ts service should import 'useWebsocketStore'
// and call 'useWebsocketStore.getState().setStatus(...)' to update the status.
// Example within websocket.ts:
// import { useWebsocketStore } from './store/websocketStore';
// ws.onopen = () => { useWebsocketStore.getState().setStatus('Connected'); };
// ws.onclose = () => { useWebsocketStore.getState().setStatus('Disconnected'); };
// ws.onerror = () => { useWebsocketStore.getState().setStatus('Error'); }; 