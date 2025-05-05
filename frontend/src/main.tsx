import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'

// 1. Import Roboto Font
import '@fontsource/roboto/300.css'
import '@fontsource/roboto/400.css'
import '@fontsource/roboto/500.css'
import '@fontsource/roboto/700.css'

// 2. Import theme provider context
import { ThemeStoreProvider } from './lib/store/themeContext.tsx'
import CssBaseline from '@mui/material/CssBaseline'

// 不再需要ThemedApp组件，改用Provider模式
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeStoreProvider>
      <CssBaseline />
    <App />
    </ThemeStoreProvider>
  </StrictMode>,
)
