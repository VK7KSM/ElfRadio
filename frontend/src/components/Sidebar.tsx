import React from 'react';
import {
  Box,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  ListSubheader,
  Divider,
  Typography,
  IconButton,
} from '@mui/material';
import ChatIcon from '@mui/icons-material/Chat';
import AirplanemodeActiveIcon from '@mui/icons-material/AirplanemodeActive';
import SatelliteAltIcon from '@mui/icons-material/SatelliteAlt';
import SosIcon from '@mui/icons-material/Sos'; // Using Sos for Emergency
import RouterIcon from '@mui/icons-material/Router'; // For APRS and Meshtastic
import SchoolIcon from '@mui/icons-material/School';
import HistoryIcon from '@mui/icons-material/History'; // For Task History
import ContactsIcon from '@mui/icons-material/Contacts'; // For Address Book
import ListAltIcon from '@mui/icons-material/ListAlt'; // For Call Log
import SettingsIcon from '@mui/icons-material/Settings';
import ArticleIcon from '@mui/icons-material/Article'; // For View Logs and Docs/GitHub
import LanguageIcon from '@mui/icons-material/Language';
import GitHubIcon from '@mui/icons-material/GitHub'; // Direct GitHub icon
import VolunteerActivismIcon from '@mui/icons-material/VolunteerActivism';
import Brightness4Icon from '@mui/icons-material/Brightness4'; // Dark mode icon
import Brightness7Icon from '@mui/icons-material/Brightness7'; // Light mode icon
import { useThemeStore } from '../lib/store/themeStore'; // 引入主题存储

// 导入任务相关功能
import { startTask } from '../lib/services/apiService'; // 导入 startTask 函数
import { useTaskStore, TaskMode } from '../lib/store/taskStore'; // 仅导入 TaskMode 和 store

// Define the width for consistency, could be moved to a theme later
const drawerWidth = 256;

const Sidebar: React.FC = () => {
  // 任务选择状态
  const [selectedItem, setSelectedItem] = React.useState<TaskMode | null>(null);
  
  // 获取主题模式和切换函数
  const { mode, toggleMode } = useThemeStore();
  
  // 获取任务状态和 setter 函数
  const taskStatus = useTaskStore(state => state.status);
  const setSelectedMode = useTaskStore(state => state.setSelectedMode); // 需要在 taskStore 中添加

  // 处理任务模式选择
  const handleSelectMode = (mode: TaskMode) => {
    if (taskStatus === 'Idle') { // 只有在空闲状态下才允许选择
      setSelectedItem(mode);
      setSelectedMode(mode); // 更新全局状态中的选定模式
    }
  };

  return (
    <Box sx={{ width: drawerWidth, height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Logo and Title Area */}
      <Box sx={{ display: 'flex', alignItems: 'center', p: 2, flexShrink: 0 }}>
        {/* Placeholder for Logo Image - Using Emoji for now */}
        <span style={{ fontSize: '24px', marginRight: '8px' }}>📻</span>
        <Typography variant="h6" noWrap component="div">
          电台精灵 ElfRadio
        </Typography>
      </Box>
      <Divider />

      {/* Navigation List */}
      <Box sx={{ overflowY: 'auto', flexGrow: 1 }}> {/* Make the list scrollable and take available space */}
        <List component="nav" dense> {/* dense for smaller items */}
          {/* --- Task Modes --- */}
          <ListSubheader component="div">任务模式</ListSubheader>
          <ListItemButton
            selected={selectedItem === 'GeneralCommunication'}
            onClick={() => handleSelectMode('GeneralCommunication')}
            disabled={taskStatus !== 'Idle'} // 非空闲状态禁用选择
          >
            <ListItemIcon><ChatIcon /></ListItemIcon>
            <ListItemText primary="💬 普通通信" />
          </ListItemButton>
          <ListItemButton disabled>
            <ListItemIcon><AirplanemodeActiveIcon /></ListItemIcon>
            <ListItemText primary="✈️ 航空监听" secondary="开发中" />
          </ListItemButton>
          <ListItemButton disabled>
            <ListItemIcon><SatelliteAltIcon /></ListItemIcon>
            <ListItemText primary="🛰️ 卫星通信" secondary="开发中" />
          </ListItemButton>
          <ListItemButton disabled>
            <ListItemIcon><SosIcon /></ListItemIcon>
            <ListItemText primary="🆘 应急通信" secondary="开发中" />
          </ListItemButton>
          <ListItemButton disabled>
            <ListItemIcon><RouterIcon /></ListItemIcon> {/* Using RouterIcon for APRS */}
            <ListItemText primary=" APRS" secondary="开发中" />
          </ListItemButton>
          <ListItemButton disabled>
            <ListItemIcon><SchoolIcon /></ListItemIcon>
            <ListItemText primary="🎓 模拟呼叫练习" secondary="开发中" />
          </ListItemButton>
          {/* 2. Add Meshtastic Gateway Item */}
          <ListItemButton disabled>
            <ListItemIcon><RouterIcon /></ListItemIcon> {/* Using RouterIcon */}
            <ListItemText primary="📶 Meshtastic 网关" secondary="开发中" />
          </ListItemButton>

          {/* --- Data Management --- */}
          <Divider sx={{ my: 1 }} />
          <ListSubheader component="div">数据管理</ListSubheader>
           {/* 3. Adjusted Data Management items */}
          <ListItemButton disabled>
            <ListItemIcon><HistoryIcon /></ListItemIcon>
            <ListItemText primary="= 历史任务" secondary="开发中" />
          </ListItemButton>
          <ListItemButton disabled>
            <ListItemIcon><ContactsIcon /></ListItemIcon>
            <ListItemText primary="👥 通信录" secondary="开发中" />
          </ListItemButton>
          <ListItemButton disabled>
            <ListItemIcon><ListAltIcon /></ListItemIcon>
            <ListItemText primary="📖 呼叫日志" secondary="开发中" />
          </ListItemButton>

          {/* --- System --- */}
          <Divider sx={{ my: 1 }} />
          <ListSubheader component="div">系统</ListSubheader>
          <ListItemButton>
            <ListItemIcon><SettingsIcon /></ListItemIcon>
            <ListItemText primary="⚙️ 设置" />
          </ListItemButton>
          {/* 1. Add View Logs Item */}
          <ListItemButton>
            <ListItemIcon><ArticleIcon /></ListItemIcon>
            <ListItemText primary="#️⃣ 查看日志" />
          </ListItemButton>

          {/* --- Other --- */}
          <Divider sx={{ my: 1 }} />
          <ListSubheader component="div">其他</ListSubheader>
          <ListItemButton component="a" href="https://elfradio.net" target="_blank" rel="noopener noreferrer">
            <ListItemIcon><LanguageIcon /></ListItemIcon>
            <ListItemText primary="🌐 官方网站" secondary="elfradio.net"/>
          </ListItemButton>
          <ListItemButton component="a" href="https://github.com/VK7KSM/ElfRadio" target="_blank" rel="noopener noreferrer">
            <ListItemIcon><GitHubIcon /></ListItemIcon>
            <ListItemText primary="📄 GitHub 仓库/文档" />
          </ListItemButton>
          <ListItemButton disabled>
            <ListItemIcon><VolunteerActivismIcon /></ListItemIcon>
            <ListItemText primary="💰 支持项目" secondary="待添加" />
          </ListItemButton>
        </List>
      </Box>
      
      {/* 主题切换按钮 - 放在底部 */}
      <Box sx={{ p: 2, mt: 'auto' }}>
        <Divider sx={{ mb: 1 }}/>
        <Box sx={{ display: 'flex', alignItems: 'center' }}>
          <IconButton onClick={toggleMode} color="inherit">
            {mode === 'dark' ? <Brightness7Icon /> : <Brightness4Icon />}
          </IconButton>
          <Typography variant="caption" sx={{ ml: 1 }}>
            {mode === 'dark' ? '浅色模式' : '深色模式'}
          </Typography>
        </Box>
      </Box>
    </Box>
  );
};

export default Sidebar; 