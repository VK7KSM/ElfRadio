import React from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import Paper from '@mui/material/Paper';
import Link from '@mui/material/Link';
import IconButton from '@mui/material/IconButton';
import { useTheme } from '@mui/material/styles';
import { useNavigate } from 'react-router-dom';
import logoLight from '../assets/logo200.png';
import logoDark from '../assets/logo200dark.png';
import SmartToyRounded from '@mui/icons-material/SmartToyRounded';
import CellTowerRounded from '@mui/icons-material/CellTowerRounded';
import SecurityRounded from '@mui/icons-material/SecurityRounded';
import SettingsInputAntennaRounded from '@mui/icons-material/SettingsInputAntennaRounded';
import DnsRoundedIcon from '@mui/icons-material/DnsRounded';
import GitHubIcon from '@mui/icons-material/GitHub';
import TelegramIcon from '@mui/icons-material/Telegram';
import EmailOutlinedIcon from '@mui/icons-material/EmailOutlined';
import LanguageIcon from '@mui/icons-material/Language';

/**
 * HomePage 组件
 * 作为应用的主仪表盘或首页。
 * 根据 page-2.jpg 设计实现了LOGO和Slogan的显示。
 * 现在包含可交互的功能入口和导航功能，优化了交互效果避免布局偏移。
 */
const HomePage: React.FC = () => {
  const theme = useTheme();
  const navigate = useNavigate();

  // TODO: Boss 指示动态效果应与左侧导航栏类似或更优。以下颜色定义参考 Sidebar.tsx 中的 currentColors，为功能入口定义合适的悬停和点击状态下的视觉效果，确保与主题协调。
  const entryColors = {
    hover: {
      backgroundColor: theme.palette.mode === 'dark' 
        ? 'rgba(54, 74, 79, 0.3)' // 参考侧边栏暗色模式悬停色，但更透明
        : 'rgba(196, 221, 226, 0.3)', // 参考侧边栏亮色模式悬停色，但更透明
      borderColor: theme.palette.mode === 'dark'
        ? '#E3E2E6' // 参考侧边栏暗色模式悬停边框色
        : '#213547', // 参考侧边栏亮色模式悬停边框色
    },
    active: {
      backgroundColor: theme.palette.mode === 'dark'
        ? 'rgba(54, 74, 79, 0.5)' // 更深的背景色表示按下状态
        : 'rgba(188, 211, 216, 0.5)',
    }
  };

  // 定义五个主要功能入口数据（已添加路径属性用于导航）
  const functionEntries = [
    { 
      icon: <SmartToyRounded sx={{ fontSize: '4rem', mb: 1.5 }} />, 
      label: '模拟呼叫练习', 
      path: '/simulated-qso' 
    },
    { 
      icon: <CellTowerRounded sx={{ fontSize: '4rem', mb: 1.5 }} />, 
      label: '普通呼叫任务', 
      path: '/general-call' 
    },
    { 
      icon: <SecurityRounded sx={{ fontSize: '4rem', mb: 1.5 }} />, 
      label: '应急通信管理', 
      path: '/emergency' 
    },
    { 
      icon: <SettingsInputAntennaRounded sx={{ fontSize: '4rem', mb: 1.5 }} />, 
      label: 'Meshtastic', 
      path: '/meshtastic' 
    },
    { 
      icon: <DnsRoundedIcon sx={{ fontSize: '4rem', mb: 1.5 }} />, 
      label: 'SDR服务器', 
      path: '/sdr-server' 
    },
  ];

  // 处理功能入口点击事件
  const handleEntryClick = (entry: { label: string; path: string }) => {
    console.log(`功能入口被点击: ${entry.label}, 导航到: ${entry.path}`);
    navigate(entry.path);
  };

  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'space-between',
        height: '100%',
        p: 3,
      }}
    >
      {/* 主要内容区域：Logo、标语和功能入口 */}
      <Box
        sx={{
          flexGrow: 1,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          width: '100%',
        }}
      >
        <Box sx={{ mb: 3 }}>
          <img
            src={theme.palette.mode === 'dark' ? logoDark : logoLight}
            alt="ElfRadio Logo"
            style={{ width: '200px', height: '166px' }}
          />
        </Box>

        {/* 主标语（中文） */}
        <Typography variant="h5" component="p" sx={{ textAlign: 'center', mb: 1 }}>
          AI 驱动的业余无线电学习与操作平台
        </Typography>

        {/* 主标语（英文） */}
        <Typography 
          variant="body2" 
          component="p" 
          sx={{ 
            color: theme.palette.text.disabled,
            fontStyle: 'italic',
            borderTop: `1px solid ${theme.palette.divider}`,
            paddingTop: theme.spacing(0.5),
            textAlign: 'center', 
            mb: 2,
            letterSpacing: '0.02em',
          }}
        >
          AI-Powered Amateur Radio Learning & Operation Platform
        </Typography>

        {/* 副标语（中文，已移除句号） */}
        <Typography 
          variant="body1" 
          component="p" 
          sx={{ 
            textAlign: 'center', 
            color: 'text.secondary', 
            maxWidth: '500px', 
            mb: 1,
          }}
        >
          为你的无线电设备装入AI的大脑，助你通联到全世界的每个人
        </Typography>

        {/* 副标语（英文） */}
        <Typography 
          variant="body2" 
          component="p" 
          sx={{ 
            color: theme.palette.text.disabled,
            fontStyle: 'italic',
            borderTop: `1px solid ${theme.palette.divider}`,
            paddingTop: theme.spacing(0.5),
            textAlign: 'center', 
            maxWidth: '520px',
            fontSize: '0.9rem',
            letterSpacing: '0.01em',
          }}
        >
          Empower Your Radio with an AI Brain, Connect with Everyone Worldwide
        </Typography>

        {/* 主要功能入口区域 - 优化交互效果，避免布局偏移 */}
        <Box sx={{ mt: 6, width: '100%' }}>
          <Stack
            direction="row"
            spacing={4}
            justifyContent="center"
            alignItems="flex-start"
            useFlexGap
            flexWrap="wrap"
            sx={{ maxWidth: 'lg', margin: '0 auto', px: 2 }}
          >
            {functionEntries.map((entry, index) => (
              <Paper
                key={index}
                elevation={2}
                onClick={() => handleEntryClick(entry)}
                sx={{
                  p: 3,
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  textAlign: 'center',
                  width: { 
                    xs: 'calc(100% - 32px)', 
                    sm: 'calc(50% - 32px)', 
                    md: 'calc(20% - 32px)' 
                  },
                  minWidth: 140,
                  minHeight: 170, // 增加固定高度以容纳缩放效果
                  height: 170,    // 确保固定高度，防止布局偏移
                  boxSizing: 'border-box',
                  cursor: 'pointer',
                  position: 'relative', // 为内部缩放元素提供定位上下文
                  // Paper 本身只改变非布局相关的属性
                  transition: theme.transitions.create(
                    ['box-shadow', 'background-color', 'border-color'], 
                    { duration: theme.transitions.duration.shorter }
                  ),
                  // 悬停效果 - 不改变 Paper 尺寸，只改变视觉属性
                  '&:hover': {
                    boxShadow: theme.shadows[6],
                    backgroundColor: entryColors.hover.backgroundColor,
                    border: `2px solid ${entryColors.hover.borderColor}`,
                    // 图标颜色在悬停时的变化
                    '& .MuiSvgIcon-root': {
                      color: entryColors.hover.borderColor,
                    },
                    // 触发内部包装器的缩放效果
                    '& .interactive-content-wrapper': {
                      transform: 'scale(1.03)',
                    },
                  },
                  // 点击/激活效果
                  '&:active': {
                    boxShadow: theme.shadows[3],
                    backgroundColor: entryColors.active.backgroundColor,
                    // 触发内部包装器的缩放效果
                    '& .interactive-content-wrapper': {
                      transform: 'scale(0.98)',
                    },
                  },
                }}
              >
                {/* 交互内容包装器 - 负责缩放效果 */}
                <Box
                  className="interactive-content-wrapper"
                  sx={{
                    display: 'flex',
                    flexDirection: 'column',
                    alignItems: 'center',
                    justifyContent: 'center',
                    width: '100%',
                    height: '100%',
                    // 只对 transform 应用过渡效果
                    transition: theme.transitions.create(['transform'], { 
                      duration: theme.transitions.duration.shorter 
                    }),
                  }}
                >
                  {/* 功能入口图标 */}
                  {entry.icon}
                  <Typography variant="subtitle1" component="h3" sx={{ mt: 1 }}>
                    {entry.label}
                  </Typography>
                </Box>
              </Paper>
            ))}
          </Stack>
        </Box>
      </Box>

      {/* 联系我们区域 - 现在位于页面底部 */}
      <Box sx={{ mt: 'auto', mb: 2, textAlign: 'center' }}>
        <Typography variant="h6" sx={{ mb: 2 }}>
          联系我们：
        </Typography>
        <Stack
          direction="row"
          spacing={3}
          justifyContent="center"
          alignItems="center"
          flexWrap="wrap"
        >
          <Link
            href="https://github.com/VK7KSM/ElfRadio"
            target="_blank"
            rel="noopener noreferrer"
            color="inherit"
          >
            <IconButton color="inherit">
              <GitHubIcon />
            </IconButton>
          </Link>
          <Link
            href="https://t.me/e1vix"
            target="_blank"
            rel="noopener noreferrer"
            color="inherit"
          >
            <IconButton color="inherit">
              <TelegramIcon />
            </IconButton>
          </Link>
          <Link
            href="mailto:Hello@elfRadio.net"
            target="_blank"
            rel="noopener noreferrer"
            color="inherit"
          >
            <IconButton color="inherit">
              <EmailOutlinedIcon />
            </IconButton>
          </Link>
          <Link
            href="https://www.elfradio.net"
            target="_blank"
            rel="noopener noreferrer"
            color="inherit"
          >
            <IconButton color="inherit">
              <LanguageIcon />
            </IconButton>
          </Link>
        </Stack>
      </Box>
    </Box>
  );
};

export default HomePage;
