import React, { StrictMode, useEffect } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App'

// 1. Import Roboto Font
import '@fontsource/roboto/300.css'
import '@fontsource/roboto/400.css'
import '@fontsource/roboto/500.css'
import '@fontsource/roboto/700.css'

// 2. MUI 和主题相关导入
import { ThemeProvider, CssBaseline } from '@mui/material'
import { lightTheme, darkTheme } from './theme'
import { useThemeStore } from './lib/store/themeStore'

// 不再需要旧的 ThemeStoreProvider (如果它与 MUI ThemeProvider 功能重复)
// import { ThemeStoreProvider } from './lib/store/themeContext.tsx'

const MainWrapper: React.FC = () => {
  const themeMode = useThemeStore((state) => state.mode)
  const setThemeMode = useThemeStore((state) => state.setThemeMode)
  const activeTheme = themeMode === 'light' ? lightTheme : darkTheme

  useEffect(() => {
    // Helper function to get system preference
    const getSystemPref = (): 'light' | 'dark' => {
      if (typeof window !== 'undefined' && window.matchMedia) {
        return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
      }
      return 'light' // Default if not in browser
    }

    // 主题初始化日志
    // Zustand 的 persist中间件已经处理了从 localStorage 加载或使用 getSystemPreference() 进行初始化。
    // 我们可以在潜在的 rehydration 之后记录状态以获得清晰度。
    const initialStoreMode = useThemeStore.getState().mode // 获取已初始化的 store 状态
    const initialSystemPref = getSystemPref()
    console.log(`主题初始化完成。Store Mode (可能来自 localStorage): ${initialStoreMode}, 当前系统偏好: ${initialSystemPref}`)

    // OS Theme Change Listener Logic
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    
    const handleChange = (e: MediaQueryListEvent) => {
      const newSystemPref = e.matches ? 'dark' : 'light'
      console.log(`检测到操作系统主题更改为: ${newSystemPref}。应用主题将强制同步到此新的系统偏好。`)
      // 始终更新主题以匹配新的系统偏好。
      // 这也将通过 useThemeStore 中的 persist 中间件更新 localStorage。
      setThemeMode(newSystemPref)
    }

    mediaQuery.addEventListener('change', handleChange)
    
    // Cleanup listener on component unmount
    return () => {
      mediaQuery.removeEventListener('change', handleChange)
      console.log('移除了操作系统主题更改监听器。')
    }
  }, [setThemeMode]) // 依赖于从 store 的闭包中获取的 setThemeMode。
                     // 如果 setThemeMode 本身是稳定的（Zustand 中应该是这样），
                     // 这个 effect 会在挂载时运行一次，并在卸载时清理。

  return (
    <ThemeProvider theme={activeTheme}>
      <CssBaseline />
      <App />
    </ThemeProvider>
  )
}

const container = document.getElementById('root')
if (container) {
  const root = createRoot(container)
  root.render(
    <StrictMode>
      <MainWrapper />
    </StrictMode>
  )
} else {
  console.error('未能找到用于 React 应用的根元素。')
}
