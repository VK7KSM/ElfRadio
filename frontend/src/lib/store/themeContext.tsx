import React, { createContext, useContext, ReactNode } from 'react';
import { ThemeProvider } from '@mui/material/styles';
import { useThemeStore } from './themeStore';
import { lightTheme, darkTheme } from '../../theme';

// 创建一个空的上下文
const ThemeStoreContext = createContext(null);

// 提供一个Provider组件
export function ThemeStoreProvider({ children }: { children: ReactNode }) {
  // 这里使用hook是安全的，因为这是在组件内部
  const themeMode = useThemeStore((state) => state.mode);
  const theme = themeMode === 'dark' ? darkTheme : lightTheme;

  return (
    <ThemeStoreContext.Provider value={null}>
      <ThemeProvider theme={theme}>
        {children}
      </ThemeProvider>
    </ThemeStoreContext.Provider>
  );
}

// 可选：提供一个自定义Hook来使用上下文
export const useThemeContext = () => useContext(ThemeStoreContext);
