import React, { useEffect } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Divider from '@mui/material/Divider';
import Pagination from '@mui/material/Pagination';
import { Routes, Route, Navigate, useLocation } from 'react-router-dom';
import { useTheme, alpha } from '@mui/material/styles'; // 导入 useTheme 以访问主题对象，确保 alpha 已导入
import AddCircleOutlineRoundedIcon from '@mui/icons-material/AddCircleOutlineRounded'; // 新增图标导入

// 导入子组件 (这些组件将在后续步骤中创建/重构)
import Sidebar from './components/Sidebar';
import StatusBar from './components/StatusBar';

// 导入页面组件
import HomePage from './pages/HomePage';
import SimulatedQSOPage from './pages/SimulatedQSOPage';
import GeneralCallTaskPage from './pages/GeneralCallTaskPage';
import EmergencyPage from './pages/EmergencyPage';
import MeshtasticPage from './pages/MeshtasticPage';
import SDRServerPage from './pages/SDRServerPage';
import ContactsPage from './pages/ContactsPage';
import SettingsPage from './pages/SettingsPage';

// 为初始 effect 导入所需的服务和 store
import { initializeWebSocket } from './lib/services/websocketService'; // 假设此文件和函数存在
import { checkBackendHealth } from './lib/services/apiService'; // 假设此文件和函数存在
import { useBackendStatusStore } from './lib/store/backendStatusStore'; // 假设此 store 存在
// import { useTaskStore } from './lib/store/taskStore'; // 如果暂时未在 App.tsx 中直接使用，则保持注释

function App() {
  const theme = useTheme(); // 获取当前主题对象以访问调色板
  const location = useLocation(); // 获取当前路由位置

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

  // 判断是否为首页路由
  const isHomePage = location.pathname === '/home' || location.pathname === '/';

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
          {/* 板块2: 次级板块 (任务列表 / 子菜单) - 条件性渲染 */}
          {!isHomePage && (
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
              }}
            >
              {/* 根据当前路由动态渲染二级板块内容 */}
              {location.pathname === '/general-call' ? (
                // "普通呼叫任务"的特定二级面板内容 - 第六次设计迭代 (极致细节优化)
                <Box 
                  sx={{ 
                    display: 'flex', 
                    flexDirection: 'column', 
                    height: '100%', 
                    p: theme.spacing(2), // 16px 统一内边距
                    boxSizing: 'border-box',
                    overflowY: 'auto', // 使整个二级面板内容可滚动
                    // 自定义滚动条样式 - 应用于此Box
                    '&::-webkit-scrollbar': { width: '5px' }, // 更窄的滚动条
                    '&::-webkit-scrollbar-track': { background: 'transparent' },
                    '&::-webkit-scrollbar-thumb': { 
                      backgroundColor: '#a1efff', // 与按钮颜色一致
                      borderRadius: '10px' 
                    },
                    '&::-webkit-scrollbar-thumb:hover': { 
                      backgroundColor: '#85e3f2', // 悬停颜色加深
                    }
                  }}
                >
                  {/* 1. 功能标题 */}
                  <Typography 
                    variant="h4" 
                    component="h1"
                    sx={{ 
                      fontWeight: 500, 
                      textAlign: 'center', 
                      mt: theme.spacing(2),   // 增加顶部间距，约16px
                      mb: theme.spacing(3),   // 24px
                    }}
                  >
                    普通呼叫任务
                  </Typography>

                  {/* 2. 添加新任务按钮 */}
                  <Button 
                    variant="contained" 
                    fullWidth 
                    startIcon={<AddCircleOutlineRoundedIcon />}
                    sx={{ 
                      mb: theme.spacing(3.5), 
                      py: theme.spacing(1.25), 
                      borderRadius: '20px', // 改为20像素圆角
                      backgroundColor: '#a1efff', 
                      color: theme.palette.getContrastText('#a1efff'),
                      '&:hover': { 
                        backgroundColor: '#85e3f2', 
                        boxShadow: theme.shadows[2],
                        transform: 'scale(1.02)',
                      },
                      transition: theme.transitions.create(['background-color', 'box-shadow', 'transform'], {
                        duration: theme.transitions.duration.short,
                      }),
                      textTransform: 'none', 
                      fontSize: '0.9375rem',
                      fontWeight: 500,
                    }}
                  >
                    添加新任务
                  </Button>
                  
                  {/* 3. 当前任务区域 */}
                  <Typography variant="subtitle1" sx={{ fontWeight: 500, color: 'text.primary', mb: 1 }}>
                    当前任务
                  </Typography>
                  <Paper 
                    elevation={1} // 统一阴影高度
                    variant="outlined" 
                    sx={{ 
                      p: theme.spacing(1.5, 2), 
                      borderRadius: '10px', // 改为10像素圆角
                      mb: theme.spacing(3), 
                    }}
                  >
                    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.25 }}>
                      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 1 }}> {/* 使用 center 对齐 */}
                        <Typography variant="body1" component="div" sx={{ fontWeight: 500, lineHeight: 1.5, flexGrow: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}> {/* 任务名称字号调小 */}
                          GC_20250526_192341Z
                        </Typography>
                        <Typography 
                          variant="caption" 
                          sx={{ 
                            color: theme.palette.success.dark, // 更深的成功色
                            backgroundColor: alpha(theme.palette.success.main, 0.15), 
                            fontWeight: 500,
                            px: 1, 
                            py: 0.35, // 调整padding使背景高度与body1协调
                            borderRadius: 1, 
                            whiteSpace: 'nowrap',
                            lineHeight: 1.5, // 确保与任务名称行高接近
                            alignSelf: 'center', // 尝试使其垂直居中于flex容器
                          }}
                        >
                          运行中
                        </Typography>
                      </Box>
                      <Typography variant="caption" color="text.disabled" sx={{ display: 'block', textAlign: 'right', mt: 0.5 }}>
                        2025-05-26 19:23:41 {/* 精确到秒 */}
                      </Typography>
                    </Box>
                  </Paper>

                  {/* 4. 分隔符 */}
                  <Divider sx={{ mb: theme.spacing(2.5) }} />

                  {/* 5. 历史任务区域 */}
                  <Typography variant="subtitle1" sx={{ fontWeight: 500, color: 'text.primary', mb: 1 }}>
                    历史任务
                  </Typography>
                  <Box sx={{ flexGrow: 1, overflowY: 'visible' }}>
                    {Array.from({ length: 3 }).map((_, index) => (
                      <Paper 
                        key={index} 
                        elevation={1} // 统一阴影高度
                        variant="outlined" 
                        sx={{ 
                          p: theme.spacing(1.5, 2), 
                          mb: theme.spacing(1.5),    
                          borderRadius: '10px', // 改为10像素圆角
                        }}
                      >
                        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 0.75 }}>
                          <Typography variant="body1" component="div" sx={{ fontWeight: 500, flexGrow: 1, mr: 1, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}> {/* 与当前任务名称字体一致 */}
                            #{index + 1}: GC_2025051{index + 2}_080530Z_这是一个非常非常长的任务名称用来测试省略号的效果是否正常显示
                          </Typography>
                          <Button 
                            size="small" 
                            variant="text" 
                            onClick={() => console.log(`Export task ${index + 1}`)}
                            sx={{ py: 0, px: 0.5, minWidth: 'auto', color: theme.palette.error.main, textTransform: 'none', fontWeight: 500, flexShrink: 0, '&:hover': { backgroundColor: alpha(theme.palette.error.main, 0.08) } }}
                          >
                            导出
                          </Button>
                        </Box>
                        {/* 摘要和"更多"按钮容器 */}
                        <Box sx={{ position: 'relative', mb: 0.5 }}>
                          <Typography 
                            variant="caption" 
                            color="text.secondary"
                            sx={{
                              lineHeight: 1.45, 
                              maxHeight: `calc(1.45em * 3)`, 
                              overflow: 'hidden',
                              display: '-webkit-box',
                              WebkitLineClamp: 3,
                              WebkitBoxOrient: 'vertical',
                            }}
                          >
                            摘要: 2025-05-1{index + 2} 这是任务 {index + 1} 的AI总结对话内容摘要占位符，包含大约中文150字或英文50个单词左右的内容，用于展示。这里是补充的文本以达到足够的长度，确保能够测试多行省略号的效果，并且内容会持续增加直到触发省略，第三行应该只显示一部分然后是更多按钮...
                          </Typography>
                          {/* "更多"按钮 - 尝试覆盖在第三行末尾 */}
                          <Button 
                            size="small" 
                            variant="text" 
                            onClick={() => console.log(`Show more for task ${index + 1}`)}
                            sx={{ 
                              p: 0, 
                              px: 0.5, // 使用与"导出"按钮相同的水平内边距
                              lineHeight: 1.45,
                              color: 'primary.main', 
                              textTransform: 'none', 
                              position: 'absolute', 
                              bottom: 0, 
                              right: 0,
                              minWidth: 'auto',
                              background: `linear-gradient(to right, transparent 0%, ${theme.palette.background.paper} 20%)`, // 恢复渐变效果
                              paddingLeft: theme.spacing(1), // 为渐变效果添加左内边距
                            }}
                          >
                            更多
                          </Button>
                        </Box>
                        <Typography variant="caption" color="text.disabled" sx={{ display: 'block', textAlign: 'right', mt: 1 }}>
                          2025-05-1{index + 2} 08:05:30
                        </Typography>
                      </Paper>
                    ))}
                  </Box>

                  {/* 6. 分页控件 */}
                  <Box sx={{ mt: 'auto', pt: theme.spacing(1.5), pb: theme.spacing(0.5), display: 'flex', justifyContent: 'center', flexShrink: 0 }}>
                    <Pagination 
                      count={5} 
                      page={1} 
                      color="primary" 
                      size="small" 
                      sx={{
                        '& .MuiPaginationItem-root': { 
                          minWidth: '28px', 
                          height: '28px',
                          fontSize: '0.8rem',
                        },
                        '& .MuiPaginationItem-root.Mui-selected': {
                          backgroundColor: '#a1efff', 
                          color: theme.palette.getContrastText('#a1efff'),
                          '&:hover': {
                            backgroundColor: '#85e3f2',
                          }
                        },
                      }}
                    />
                  </Box>
                </Box>
              ) : (
                // 其他非首页页面的默认二级面板内容
                <Box sx={{ p: 2 /* 确保其他页面的二级面板也有内边距 */}}>
                  <Typography variant="h6" component="h2">二级板块</Typography>
                  <Typography variant="body2">(任务列表/菜单)</Typography>
                  {/* 可以为其他页面定义不同的二级面板内容 */}
                </Box>
              )}
            </Box>
          )}

          {/* 板块3: 主内容区域 - 配置路由 */}
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
              // 首页时移除内边距，让 HomePage 组件自己控制布局
              ...(isHomePage ? {} : { p: 3 }),
            }}
          >
            <Routes>
              <Route path="/" element={<Navigate to="/home" replace />} />
              <Route path="/home" element={<HomePage />} />
              <Route path="/simulated-qso" element={<SimulatedQSOPage />} />
              <Route path="/general-call" element={<GeneralCallTaskPage />} />
              <Route path="/emergency" element={<EmergencyPage />} />
              <Route path="/meshtastic" element={<MeshtasticPage />} />
              <Route path="/sdr-server" element={<SDRServerPage />} />
              <Route path="/contacts" element={<ContactsPage />} />
              <Route path="/settings" element={<SettingsPage />} />
            </Routes>
          </Box>
        </Box> {/* 板块2和板块3容器结束 */}
      </Box> {/* 右侧内容容器结束 */}
    </Box> // 根 Flex 容器结束
  );
}

export default App; 