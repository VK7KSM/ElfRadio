import { create } from 'zustand';

interface AuthState {
  token: string | null;
  setToken: (token: string | null) => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  token: null, // Initially no token
  setToken: (token) => set({ token }),
}));

console.log('authStore.ts loaded'); 