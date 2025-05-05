import React, { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import Snackbar from '@mui/material/Snackbar';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import StopCircleIcon from '@mui/icons-material/StopCircle'; // 导入停止图标
import CircularProgress from '@mui/material/CircularProgress'; // 导入加载进度组件
import MessageInput from './MessageInput'; // 导入消息输入组件
import { sendTextMessage, startTask, stopTask } from '../lib/services/apiService'; // 导入发送文本API函数、启动任务API函数和停止任务API函数
import { useTaskStore, useCurrentTaskStatus, useSelectedMode, useActiveTaskInfo } from '../lib/store/taskStore'; // 导入 taskStore 和相关钩子函数
// 如果需要实现本地消息显示，可以导入 chatStore
// import { useChatStore } from '../lib/store/chatStore';

const ChatView: React.FC = () => {
  // 状态管理
  const [isLoading, setIsLoading] = useState(false);
  const [startTaskLoading, setStartTaskLoading] = useState(false);
  const [stopTaskLoading, setStopTaskLoading] = useState(false); // 新增停止任务加载状态
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [showError, setShowError] = useState(false);

  // 从 taskStore 获取数据
  const taskStatus = useCurrentTaskStatus();
  const selectedMode = useSelectedMode();
  const activeTaskInfo = useActiveTaskInfo();
  const { setTaskRunning, setTaskIdle, setTaskStopping } = useTaskStore.getState();

  // 处理启动任务
  const handleStartTask = async () => {
    if (!selectedMode) return;
    
    setStartTaskLoading(true);
    setErrorMessage(null);
    
    try {
      const response = await startTask(selectedMode);
      console.log('启动任务成功:', response);
      
      // 创建任务信息并更新状态
      const taskInfo = {
        id: response.task_id,
        name: `${selectedMode}_${new Date().toISOString().substring(0, 16)}`,
        mode: selectedMode
      };
      
      setTaskRunning(taskInfo);
    } catch (error) {
      console.error('启动任务失败:', error);
      setTaskIdle(); // 重置状态
      
      // 显示错误消息
      if (error instanceof Error) {
        setErrorMessage(error.message);
      } else {
        setErrorMessage('启动任务时发生未知错误');
      }
      setShowError(true);
    } finally {
      setStartTaskLoading(false);
    }
  };

  // 处理停止任务
  const handleStopTask = async () => {
    setStopTaskLoading(true);
    setErrorMessage(null); // 清除之前的错误
    try {
      await stopTask();
      console.log('停止任务请求成功');
      setTaskStopping(); // 设置状态为Stopping
    } catch (error) {
      console.error('停止任务失败:', error);
      if (error instanceof Error) {
        setErrorMessage(error.message);
      } else {
        setErrorMessage('停止任务时发生未知错误');
      }
      setShowError(true); // 显示错误提示
    } finally {
      setStopTaskLoading(false);
    }
  };

  // 处理发送消息
  const handleSend = async (text: string) => {
    console.log('发送消息:', text);
    
    try {
      setIsLoading(true);
      
      await sendTextMessage(text);
      console.log('消息通过API成功发送');
    } catch (error) {
      console.error('发送消息失败:', error);
      
      // 显示错误消息
      if (error instanceof Error) {
        setErrorMessage(error.message);
      } else {
        setErrorMessage('发送消息时发生未知错误');
      }
      setShowError(true);
    } finally {
      setIsLoading(false);
    }
  };

  // 处理关闭错误提示
  const handleCloseError = () => {
    setShowError(false);
  };

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      {/* 顶部任务信息栏 */}
      <Box sx={{ p: 1, borderBottom: 1, borderColor: 'divider', bgcolor: 'background.paper', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Typography variant="subtitle1" fontWeight="bold">
          {taskStatus === 'Running' && activeTaskInfo 
            ? `任务会话: ${activeTaskInfo.mode}` 
            : '任务会话'}
        </Typography>
        
        <Box sx={{ display: 'flex', gap: 1 }}>
          {/* 显示启动任务按钮（仅当状态为空闲且有选定模式时） */}
          {taskStatus === 'Idle' && selectedMode && (
            <Button 
              variant="contained" 
              color="primary" 
              size="small"
              startIcon={<PlayArrowIcon />}
              onClick={handleStartTask}
              disabled={startTaskLoading}
            >
              {startTaskLoading ? '启动中...' : '启动任务'}
            </Button>
          )}

          {/* 显示停止任务按钮（仅当状态为运行中时） */}
          {taskStatus === 'Running' && (
            <Button 
              variant="outlined" 
              color="error" 
              size="small"
              startIcon={stopTaskLoading ? <CircularProgress size={16} color="error" /> : <StopCircleIcon />}
              onClick={handleStopTask}
              disabled={stopTaskLoading}
            >
              {stopTaskLoading ? '停止中...' : '停止任务'}
            </Button>
          )}
        </Box>
      </Box>
      
      {/* 消息列表区域 */}
      <Box sx={{ flexGrow: 1, overflowY: 'auto', p: 2 }}>
        {/* 当有选定模式但尚未启动任务时显示提示 */}
        {taskStatus === 'Idle' && selectedMode && (
          <Alert severity="info" sx={{ mb: 2 }}>
            已选择 {selectedMode} 模式，点击"启动任务"按钮开始。
          </Alert>
        )}
        
        {/* 当没有选定模式时显示提示 */}
        {taskStatus === 'Idle' && !selectedMode && (
          <Alert severity="info" sx={{ mt: 2 }}>
            请在左侧边栏选择一个任务模式。
          </Alert>
        )}
        
        {/* 当任务正在停止时显示提示 */}
        {taskStatus === 'Stopping' && (
          <Alert severity="warning" sx={{ mt: 2 }}>
            正在停止任务，请稍候...
          </Alert>
        )}
        
        {/* 消息区 */}
        {taskStatus !== 'Idle' && (
          <Typography variant="body2" color="text.secondary" sx={{ textAlign: 'center', mt: 4 }}>
            消息将在此处显示
          </Typography>
        )}
      </Box>
      
      {/* 消息输入区域 - 仅当任务正在运行时显示 */}
      {taskStatus === 'Running' && (
        <MessageInput onSend={handleSend} isLoading={isLoading} />
      )}
      
      {/* 错误提示 */}
      <Snackbar 
        open={showError} 
        autoHideDuration={6000} 
        onClose={handleCloseError}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
      >
        <Alert onClose={handleCloseError} severity="error" sx={{ width: '100%' }}>
          {errorMessage}
        </Alert>
      </Snackbar>
    </Box>
  );
};

export default ChatView;
