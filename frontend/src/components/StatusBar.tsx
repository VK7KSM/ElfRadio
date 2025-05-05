import React from 'react';
import {
  AppBar,
  Toolbar,
  Box,
  Typography,
  CircularProgress, // 用于加载状态
} from '@mui/material';
import CheckCircleOutlineIcon from '@mui/icons-material/CheckCircleOutline'; // 成功/连接状态
import ErrorOutlineIcon from '@mui/icons-material/ErrorOutline'; // 错误/断开状态
import CloudOffIcon from '@mui/icons-material/CloudOff'; // 可选：用于后端离线状态
import CloudQueueIcon from '@mui/icons-material/CloudQueue'; // 可选：用于后端检查中状态
import HelpOutlineIcon from '@mui/icons-material/HelpOutline'; // Icon for Unknown status

// 2. Import the hook to read WebSocket status from the store
//    (Adjust path and hook name based on your actual store implementation)
import { useWebsocketStatus } from '../lib/store/websocketStore';
// --- Import backend status store hook ---
import { useBackendStatusStore } from '../lib/store/backendStatusStore';
// 导入任务状态钩子
import { useCurrentTaskStatus, useActiveTaskInfo } from '../lib/store/taskStore';

// 应用版本常量
const appVersion: string = '0.1.0 (Dev)';

const StatusBar: React.FC = () => {
  // 2. Get the live WebSocket status from the store
  const wsStatus = useWebsocketStatus();
  // --- Get live backend status from the store ---
  const backendStatus = useBackendStatusStore((state) => state.status);
  // 获取当前任务状态
  const taskStatus = useCurrentTaskStatus();
  const activeTaskInfo = useActiveTaskInfo();

  // 构建显示任务状态文本
  const displayTaskStatus = activeTaskInfo && taskStatus === 'Running'
    ? `运行中: ${activeTaskInfo.mode}` // 运行时显示模式
    : taskStatus === 'Idle' ? '空闲' : // 显示"空闲"或"正在停止"
      taskStatus === 'Stopping' ? '正在停止' : 
      taskStatus; // 兜底方案

  // 根据 WebSocket 状态返回对应的图标
  const renderWsIcon = () => {
    switch (wsStatus) {
      case 'Connecting':
        return <CircularProgress size={14} sx={{ mr: 0.5 }} color="inherit" />;
      case 'Connected':
        return <CheckCircleOutlineIcon sx={{ fontSize: 16, mr: 0.5, color: 'success.main' }} />;
      case 'Disconnected':
      case 'Error': // Group Error with Disconnected for icon display
      default:
        return <ErrorOutlineIcon sx={{ fontSize: 16, mr: 0.5, color: 'error.main' }} />;
    }
  };

  // 根据后端状态返回对应的图标
  const renderBackendIcon = () => {
    switch (backendStatus) {
      case 'Checking':
        return <CloudQueueIcon sx={{ fontSize: 16, mr: 0.5, color: 'info.main' }} />;
      case 'OK':
        return <CheckCircleOutlineIcon sx={{ fontSize: 16, mr: 0.5, color: 'success.main' }} />;
      case 'Error':
        return <ErrorOutlineIcon sx={{ fontSize: 16, mr: 0.5, color: 'error.main' }} />;
      case 'Unknown':
      default:
        return <HelpOutlineIcon sx={{ fontSize: 16, mr: 0.5, color: 'warning.main' }} />;
    }
  };

  return (
    // 使用 AppBar 固定在底部
    <AppBar
      position="fixed"
      color="default" // 可以根据主题调整颜色，'default' 通常是浅灰色
      sx={{ top: 'auto', bottom: 0, zIndex: (theme) => theme.zIndex.drawer + 1 }} // 确保在 Drawer 之上（如果 Drawer 在 AppBar 下方）
    >
      {/* 使用 dense 工具栏减少高度 */}
      <Toolbar variant="dense">
        <Box sx={{ display: 'flex', alignItems: 'center', width: '100%', gap: 2 }}> {/* 使用 gap 控制间距 */}

          {/* WebSocket 状态 */}
          <Box sx={{ display: 'flex', alignItems: 'center' }}>
            {renderWsIcon()}
            <Typography variant="caption" sx={{ lineHeight: 1 }}>WS: {wsStatus}</Typography>
          </Box>

          {/* 后端状态 */}
          <Box sx={{ display: 'flex', alignItems: 'center' }}>
            {renderBackendIcon()}
            <Typography variant="caption" sx={{ lineHeight: 1 }}>BK: {backendStatus}</Typography>
          </Box>

          {/* 任务状态 - 现在使用实际状态 */}
          <Typography variant="caption" sx={{ lineHeight: 1 }}>任务: {displayTaskStatus}</Typography>

          {/* 伸缩占位符，将版本信息推到右侧 */}
          <Box sx={{ flexGrow: 1 }} />

          {/* 版本信息 */}
          <Typography variant="caption" sx={{ lineHeight: 1 }}>
            ElfRadio {appVersion} | © 2025 VK7KSM
          </Typography>
        </Box>
      </Toolbar>
    </AppBar>
  );
};

export default StatusBar; 