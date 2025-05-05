import { create } from 'zustand';

type ThemeMode = 'light' | 'dark';

interface ThemeState {
  mode: ThemeMode;
  toggleMode: () => void;
  setMode: (mode: ThemeMode) => void; // Keep setMode for potential future use or direct setting
}

// Function to get initial mode based on system preference
const getInitialMode = (): ThemeMode => {
  if (typeof window !== 'undefined' && window.matchMedia) {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return 'light'; // Default to light if preference cannot be detected (e.g., SSR)
};

export const useThemeStore = create<ThemeState>((set) => ({
  mode: getInitialMode(), // Initialize with system preference
  toggleMode: () => set((state) => ({ mode: state.mode === 'light' ? 'dark' : 'light' })),
  setMode: (mode) => set({ mode }),
}));

// Optional: Export a selector hook for convenience
export const useCurrentThemeMode = () => useThemeStore((state) => state.mode); 