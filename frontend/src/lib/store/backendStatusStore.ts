import { create } from 'zustand';

type BackendStatus = 'OK' | 'Error' | 'Checking' | 'Unknown';

interface BackendStatusState {
  status: BackendStatus;
  setStatus: (status: BackendStatus) => void;
}

export const useBackendStatusStore = create<BackendStatusState>((set) => ({
  status: 'Unknown', // Initial state
  setStatus: (status) => set({ status }),
}));

// Optional selector hook
export const useCurrentBackendStatus = () => useBackendStatusStore((state) => state.status);

console.log('backendStatusStore.ts loaded'); // Add log to confirm file load 