import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware'; // For localStorage persistence

type ThemeMode = 'light' | 'dark';

interface ThemeState {
  mode: ThemeMode;
  toggleTheme: () => void;
  setThemeMode: (mode: ThemeMode) => void; // Allow setting a specific mode
}

// Function to get system preference
const getSystemPreference = (): ThemeMode => {
  // 确保在浏览器环境执行
  if (typeof window !== 'undefined' && window.matchMedia) {
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      return 'dark';
    }
  }
  return 'light'; // 如果无法检测或在非浏览器环境，默认为亮色
};

export const useThemeStore = create<ThemeState>()(
  persist(
    (set) => ({
      mode: getSystemPreference(), // 使用系统偏好进行初始化
      toggleTheme: () =>
        set((state) => ({
          mode: state.mode === 'light' ? 'dark' : 'light',
        })),
      setThemeMode: (newMode) => set({ mode: newMode }),
    }),
    {
      name: 'elfradio-theme-storage', // localStorage 中的项目名称
      storage: createJSONStorage(() => localStorage), // 使用 localStorage
      onRehydrateStorage: () => (state) => {
        // 此函数在从 localStorage 重新水合存储时调用。
        // 如果没有持久化主题，或者如果我们希望在首次加载时
        // 如果没有存储任何内容，则重新检查系统偏好设置，可以在此处添加逻辑。
        // 目前，如果 'mode' 在存储中，则会使用它。
        // 如果没有，则将使用初始状态 (getSystemPreference())。
        if (state) {
          console.log('主题已从 localStorage 重新水合:', state.mode);
        } else {
          // 如果 localStorage 中没有存储状态，确保使用当前系统偏好
          // (尽管 persist 中间件的 initialState 应该已经处理了这一点)
          // useThemeStore.setState({ mode: getSystemPreference() }); // 可以在这里强制设定
          console.log('localStorage 中未找到主题，使用系统偏好初始化。');
        }
      },
    }
  )
);

// 可选：导出选择器钩子以方便使用 (如果需要的话)
// export const useCurrentThemeMode = () => useThemeStore((state) => state.mode); 