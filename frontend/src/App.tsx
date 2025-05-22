import React, { useEffect } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { useTheme } from '@mui/material/styles'; // 导入 useTheme 以访问主题对象

// 导入子组件 (这些组件将在后续步骤中创建/重构)
import Sidebar from './components/Sidebar';
import StatusBar from './components/StatusBar';

// 为初始 effect 导入所需的服务和 store
import { initializeWebSocket } from './lib/services/websocketService'; // 假设此文件和函数存在
import { checkBackendHealth } from './lib/services/apiService'; // 假设此文件和函数存在
import { useBackendStatusStore } from './lib/store/backendStatusStore'; // 假设此 store 存在
// import { useTaskStore } from './lib/store/taskStore'; // 如果暂时未在 App.tsx 中直接使用，则保持注释

function App() {
  const theme = useTheme(); // 获取当前主题对象以访问调色板

  useEffect(() => {
    // 来自之前 App.tsx 的 WebSocket 和后端健康检查逻辑
    console.log('App 已挂载，正在初始化 WebSocket...');
    initializeWebSocket(); // 调用 WebSocket 初始化函数

    const healthCheck = async () => {
      const setBackendStatus = useBackendStatusStore.getState().setStatus;
      setBackendStatus('Checking'); // 设置后端状态为检查中
      console.log('正在执行后端健康检查...');
      try {
        const result = await checkBackendHealth(); // 调用后端健康检查 API
        if (result.trim() === 'OK') {
          setBackendStatus('OK'); // 设置后端状态为 OK
          console.log('后端健康检查: OK');
        } else {
          setBackendStatus('Error'); // 设置后端状态为错误
          console.error('后端健康检查失败: 意外响应:', result);
        }
      } catch (error) {
        setBackendStatus('Error'); // 设置后端状态为错误
        console.error('后端健康检查失败:', error);
      }
    };
    healthCheck(); // 执行健康检查
  }, []); // 空依赖数组，确保此 effect 仅在组件挂载时运行一次

  return (
    <Box sx={{ display: 'flex', flexDirection: 'row', height: '100vh', width: '100vw', overflow: 'hidden' }}>
      {/* 板块1: 主图标导航 (侧边栏) - 最左侧，全高 */}
      <Box
        component="aside"
        sx={{
          width: '90px', // 侧边栏宽度
          flexShrink: 0, // 防止侧边栏在 flex 布局中收缩
          // 直接应用 Boss 提供的 HEX 颜色值
          backgroundColor: theme.palette.mode === 'dark'
              ? '#1D2B2D' // 暗色模式侧边栏背景
              : '#E7F0F2', // 亮色模式侧边栏背景
          height: '100%', // 占据全部视窗高度
          overflowY: 'auto', // 如果内容超出则显示垂直滚动条
        }}
      >
        <Sidebar /> {/* 侧边栏组件 */}
      </Box>

      {/* 右侧内容容器 (状态栏, 板块2, 板块3) */}
      <Box
        sx={{
          flexGrow: 1, // 占据剩余的 flex 空间
          display: 'flex',
          flexDirection: 'column', // 内部元素垂直排列
          height: '100%', // 占据全部父容器高度
          overflow: 'hidden', // 防止内部内容溢出
        }}
      >
        {/* 顶部状态栏 - 在板块2和板块3之上 */}
        <Box
          sx={{
            flexShrink: 0, // 防止状态栏在 flex 布局中收缩
            // 直接应用 Boss 提供的 HEX 颜色值 (与侧边栏相同)
            backgroundColor: theme.palette.mode === 'dark'
              ? '#1D2B2D' // 暗色模式状态栏背景
              : '#E7F0F2', // 亮色模式状态栏背景
            width: '100%', // 添加: 确保此 Box 占据其父容器的全部可用宽度
            px: 0,         // 添加: 移除此 Box 可能存在的任何水平内边距
          }}
        >
          <StatusBar /> {/* 状态栏组件 */}
        </Box>

        {/* 板块2和板块3的容器 (并排) */}
        <Box sx={{ display: 'flex', flexDirection: 'row', flexGrow: 1, overflow: 'hidden' }}>
          {/* 板块2: 次级板块 (任务列表 / 子菜单) */}
          <Box
            component="nav" // 使用 'nav' 语义化标签，因其可能包含次级导航或任务列表
            sx={{
              width: '320px', // 次级板块宽度
              flexShrink: 0, // 防止收缩
              borderRight: `1px solid ${theme.palette.divider}`, // 右边框作为分隔线
              overflowY: 'auto', // 内容溢出时显示滚动条
              // 直接应用 Boss 提供的 HEX 颜色值
              backgroundColor: theme.palette.mode === 'dark'
                ? '#1C2527' // 暗色模式板块2背景
                : '#EFF5F7', // 亮色模式板块2背景
              height: '100%', // 占据全部父容器高度
              p: 2, // 内边距
            }}
          >
            <Typography variant="h6" component="h2">二级板块</Typography>
            <Typography variant="body2">(任务列表/菜单)</Typography>
            {/* 此处将填充实际的二级板块内容 */}
          </Box>

          {/* 板块3: 主内容区域 */}
          <Box
            component="main" // 使用 'main' 语义化标签
            sx={{
              flexGrow: 1, // 占据剩余的 flex 空间
              overflowY: 'auto', // 内容溢出时显示滚动条
              // 直接应用 Boss 提供的 HEX 颜色值
              backgroundColor: theme.palette.mode === 'dark'
                ? '#191C1D' // 暗色模式板块3背景
                : '#FBFCFD', // 亮色模式板块3背景
              height: '100%', // 占据全部父容器高度
              p: 3, // 内边距
            }}
          >
            <Typography variant="h5" component="h1" sx={{ mb: 2 }}>三级板块 (主内容)</Typography>
            <Typography>
              这里将根据二级板块的选择显示对话主界面、联系人列表、设置页面或帮助内容。
            </Typography>
            {/* 此处将填充实际的主内容 */}
          </Box>
        </Box> {/* 板块2和板块3容器结束 */}
      </Box> {/* 右侧内容容器结束 */}
    </Box> // 根 Flex 容器结束
  );
}

export default App; 