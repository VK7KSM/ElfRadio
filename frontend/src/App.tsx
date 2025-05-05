import React, { useEffect, useState } from 'react';
import Box from '@mui/material/Box';
import Drawer from '@mui/material/Drawer';
import Tabs from '@mui/material/Tabs';
import Tab from '@mui/material/Tab';

import Sidebar from './components/Sidebar';
import StatusBar from './components/StatusBar';
import LogDisplay from './components/LogDisplay';
import ChatView from './components/ChatView';
import { initializeWebSocket } from './lib/services/websocketService';
import { checkBackendHealth } from './lib/services/apiService';
import { useBackendStatusStore } from './lib/store/backendStatusStore';
import { useTaskStore } from './lib/store/taskStore';

const drawerWidth = 256;

function App() {
  const [tabValue, setTabValue] = useState(0);
  const taskStatus = useTaskStore(state => state.status);
  
  // 当任务启动时自动切换到会话标签
  useEffect(() => {
    if (taskStatus === 'Running') {
      setTabValue(0); // 切换到会话标签
    }
  }, [taskStatus]);

  useEffect(() => {
    console.log('App mounted, initializing WebSocket...');
    initializeWebSocket();

    const healthCheck = async () => {
      const setBackendStatus = useBackendStatusStore.getState().setStatus;
      setBackendStatus('Checking');
      console.log('Performing backend health check...');
      try {
        const result = await checkBackendHealth();
        if (result.trim() === 'OK') {
          setBackendStatus('OK');
          console.log('Backend health check: OK');
        } else {
          setBackendStatus('Error');
          console.error('Backend health check failed: Unexpected response:', result);
        }
      } catch (error) {
        setBackendStatus('Error');
        console.error('Backend health check failed:', error);
      }
    };

    healthCheck();
  }, []);

  const handleTabChange = (event: React.SyntheticEvent, newValue: number) => {
    setTabValue(newValue);
  };

  return (
    <Box sx={{ display: 'flex', minHeight: '100vh' }}>
      <Drawer
        variant="permanent"
        sx={{
          width: drawerWidth,
          flexShrink: 0,
          [`& .MuiDrawer-paper`]: { width: drawerWidth, boxSizing: 'border-box' },
        }}
      >
        <Sidebar />
      </Drawer>

      <Box
        component="main"
        sx={{
          flexGrow: 1,
          display: 'flex',
          flexDirection: 'column',
          height: '100vh',
          overflow: 'hidden',
        }}
      >
        {/* 标签导航 */}
        <Tabs 
          value={tabValue} 
          onChange={handleTabChange}
          sx={{ borderBottom: 1, borderColor: 'divider' }}
        >
          <Tab label="会话" />
          <Tab label="日志" />
        </Tabs>
        
        {/* 内容区域 */}
        <Box sx={{ 
          flexGrow: 1, 
          p: 0, 
          pb: '64px', 
          display: 'flex', 
          flexDirection: 'column',
          overflow: 'hidden',
        }}>
          {/* 会话视图 */}
          <Box 
            sx={{ 
              display: tabValue === 0 ? 'flex' : 'none', 
              flexDirection: 'column',
              height: '100%',
            }}
          >
            <ChatView />
          </Box>
          
          {/* 日志视图 */}
          <Box 
            sx={{ 
              display: tabValue === 1 ? 'flex' : 'none', 
              flexDirection: 'column',
              height: '100%',
            }}
          >
            <LogDisplay />
          </Box>
        </Box>
      </Box>

      <StatusBar />
    </Box>
  );
}

export default App;
