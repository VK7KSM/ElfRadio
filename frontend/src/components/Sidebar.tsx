import React, { useState, useEffect } from 'react';
import {
  Box,
  Tooltip,
  IconButton as MuiIconButton, // 重命名以避免潜在的命名冲突
  useTheme,
  SvgIconProps, // 用于为克隆的 SvgIcons 提供 sx 属性类型
} from '@mui/material';
import { SxProps, Theme } from '@mui/material/styles'; // 用于 SxProps 类型
// 移除未使用的导入
// import { SvgIconComponent } from '@mui/icons-material'; // 用于 NavItemIconProps

import { useThemeStore } from '../lib/store/themeStore';
import { useNavigate, useLocation } from 'react-router-dom';
// import { useTaskStore } from '../lib/store/taskStore'; // 如果后续需要用于导航逻辑，则导入

// 导入所有必要的 Material 图标（圆角变体）
import SmartToyRounded from '@mui/icons-material/SmartToyRounded';
import CellTowerRounded from '@mui/icons-material/CellTowerRounded';
import SecurityRounded from '@mui/icons-material/SecurityRounded';
import SettingsInputAntennaRounded from '@mui/icons-material/SettingsInputAntennaRounded';
import DnsRoundedIcon from '@mui/icons-material/DnsRounded';
import ContactEmergencyRounded from '@mui/icons-material/ContactEmergencyRounded';
import SettingsRounded from '@mui/icons-material/SettingsRounded';
import LockResetRounded from '@mui/icons-material/LockResetRounded';
import DarkModeRounded from '@mui/icons-material/DarkModeRounded';
import LightModeRounded from '@mui/icons-material/LightModeRounded';

// 导入亮色和暗色模式的侧边栏主 Logo
import logoLight from '../assets/logo80.png';
import logoDark from '../assets/logo80dark.png';

// 更新：定义 SidebarColorScheme 接口
interface SidebarColorScheme {
  sidebarBg: string;
  outerSelectedBg: string;
  innerDefaultFill: string;
  innerDefaultBorder: string;
  innerHoverFill_Light_Start?: string; // 亮色模式悬停渐变起始色
  innerHoverFill_Light_End?: string;   // 亮色模式悬停渐变结束色
  innerHoverFill_Dark?: string;        // 暗色模式悬停填充色 (通常为纯色)
  innerHoverBorder: string;
  innerSelectedFill_Light?: string;    // 亮色模式选中填充色
  innerSelectedFill_Dark?: string;     // 暗色模式选中填充色 (通常为纯色)
  innerSelectedBorder: string;
  iconDefault: string;
  iconHover: string;
  iconSelected: string;
}

interface NavItemIconProps {
  icon: React.ReactElement;
  tooltipTitle: string;
  isSelected: boolean;
  onClick: () => void;
  currentColors: SidebarColorScheme; // 使用新的 SidebarColorScheme 类型
}

const NavItemIcon: React.FC<NavItemIconProps> = ({
  icon,
  tooltipTitle,
  isSelected,
  onClick,
  currentColors,
}) => {
  // const theme = useTheme(); // 移除未使用的 theme 变量
  const themeMode = useThemeStore((state) => state.mode); // 获取当前主题模式

  const iconSx: SxProps<Theme> = {
    fontSize: '22px',
    transition: 'color 0.2s ease-in-out',
    // 更新：根据 isSelected 状态设置图标颜色
    color: isSelected ? currentColors.iconSelected : currentColors.iconDefault, 
  };

  const outerBoxSx: SxProps<Theme> = {
    width: '90px',
    height: '72px', 
    minHeight: '56px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
    marginBottom: '10px',
    // 更新：根据 isSelected 状态设置外部 Box 背景色
    backgroundColor: isSelected ? currentColors.outerSelectedBg : 'transparent', 
    transition: 'background-color 0.2s ease-in-out',
    '&:hover .nav-item-inner-visual': {
      transform: 'scale(1.05)',
      borderWidth: '2.0px',
      borderColor: currentColors.innerHoverBorder,
      ...(themeMode === 'light' && currentColors.innerHoverFill_Light_Start && currentColors.innerHoverFill_Light_End && {
        background: `linear-gradient(to bottom, ${currentColors.innerHoverFill_Light_Start}, ${currentColors.innerHoverFill_Light_End})`,
      }),
    },
    '&:hover .nav-item-icon-rendered-wrapper': {
      color: currentColors.iconHover,
    },
  };

  // 注意：由于 React Style props 不支持伪类或复杂的条件逻辑，
  // 我们将 innerVisualBox 的样式移至 sx prop，以便更好地处理选中和悬停状态的结合。
  // 原先的 innerVisualBoxStyle 将被替换为 innerVisualBoxSx。
  const innerVisualBoxSx: SxProps<Theme> = {
    width: '60px',
    height: '42px',
    borderRadius: '8px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    transition: 'transform 0.2s ease-in-out, background-color 0.2s ease-in-out, background 0.2s ease-in-out, border-color 0.2s ease-in-out, border-width 0.2s ease-in-out',
    // 默认状态
    backgroundColor: currentColors.innerDefaultFill, 
    border: `1.5px solid ${currentColors.innerDefaultBorder}`, 
    
    // 选中状态的样式
    ...(isSelected && {
      backgroundColor: themeMode === 'light' 
          ? currentColors.innerSelectedFill_Light 
          : currentColors.innerSelectedFill_Dark,
      borderColor: currentColors.innerSelectedBorder,
      borderWidth: '2.0px',
      transform: 'scale(1.05)',
    }),
  };
  
  const iconWithStyles = React.cloneElement(icon, { sx: iconSx } as Partial<SvgIconProps>);

  return (
    <Tooltip title={tooltipTitle} placement="right">
      <Box sx={outerBoxSx} onClick={onClick}>
        {/* 更新：将 innerVisualBox 的 style prop 改为 sx prop */}
        <Box className="nav-item-inner-visual" sx={innerVisualBoxSx}>
          <Box className="nav-item-icon-rendered-wrapper" sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'inherit', transition: 'color 0.2s ease-in-out' }}>
            {iconWithStyles}
          </Box>
        </Box>
      </Box>
    </Tooltip>
  );
};

const Sidebar: React.FC = () => {
  const muiTheme = useTheme();
  const themeMode = useThemeStore((state) => state.mode);
  // 根据当前主题模式选择合适的 Logo
  const currentLogo = themeMode === 'light' ? logoLight : logoDark;
  const toggleTheme = useThemeStore((state) => state.toggleTheme);
  const navigate = useNavigate();
  const location = useLocation();

  const [selectedItem, setSelectedItem] = useState<string>('home');

  // 更新：定义 sidebarColors 常量，包含所有最终HEX颜色值
  const sidebarColors: { light: SidebarColorScheme; dark: SidebarColorScheme } = {
    light: {
      sidebarBg: '#E7F0F2',
      outerSelectedBg: '#EFF5F7',
      innerDefaultFill: '#C4DDE2',
      innerDefaultBorder: '#4A6268',
      innerHoverFill_Light_Start: '#C4DDE2',
      innerHoverFill_Light_End: '#BCD3D8',
      innerHoverBorder: '#213547',
      innerSelectedFill_Light: '#BCD3D8',
      innerSelectedBorder: '#051f23',
      iconDefault: '#4A6268',
      iconHover: '#213547',
      iconSelected: '#051f23',
    },
    dark: {
      sidebarBg: '#1D2B2D',
      outerSelectedBg: '#1C2527',
      innerDefaultFill: '#364A4F',
      innerDefaultBorder: '#B1CBD1',
      innerHoverFill_Dark: '#364A4F',
      innerHoverBorder: '#E3E2E6',
      innerSelectedFill_Dark: '#364A4F',
      innerSelectedBorder: '#E3E2E6',
      iconDefault: '#B1CBD1',
      iconHover: '#E3E2E6',
      iconSelected: '#E3E2E6',
    },
  };
  const currentColors = sidebarColors[themeMode];

  // 路由导航处理函数
  const handleItemClick = (item: { name: string; path: string }) => {
    setSelectedItem(item.name);
    navigate(item.path);
    console.log(`Sidebar NavItem clicked: ${item.name}, navigating to: ${item.path}`);
  };

  // 根据当前路径同步选中状态
  useEffect(() => {
    const currentPath = location.pathname;
    const currentItem = navItems.find(item => item.path === currentPath);
    if (currentItem) {
      setSelectedItem(currentItem.name);
    } else if (currentPath === '/home' || currentPath === '/') {
      setSelectedItem('home');
    }
  }, [location.pathname]);

  // 更新：添加路径属性到 navItems 数组
  const navItems = [
    { name: 'simulated-qso', icon: <SmartToyRounded />, tooltip: '模拟呼叫练习', path: '/simulated-qso' },
    { name: 'general-call', icon: <CellTowerRounded />, tooltip: '普通呼叫任务', path: '/general-call' },
    { name: 'emergency', icon: <SecurityRounded />, tooltip: '应急通信管理', path: '/emergency' },
    { name: 'meshtastic', icon: <SettingsInputAntennaRounded />, tooltip: 'Meshtastic', path: '/meshtastic' },
    { name: 'sdr-server', icon: <DnsRoundedIcon />, tooltip: 'SDR 服务器', path: '/sdr-server' },
    { name: 'contacts', icon: <ContactEmergencyRounded />, tooltip: '通信录', path: '/contacts' },
    { name: 'system-settings', icon: <SettingsRounded />, tooltip: '系统设置', path: '/settings' },
    { name: 'lock-screen', icon: <LockResetRounded />, tooltip: '锁屏', path: '/lock-screen' },
  ];

  return (
    <Box
      sx={{
        width: '90px',
        height: '100vh',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        paddingTop: muiTheme.spacing(2),
        paddingBottom: '140px',
        boxSizing: 'border-box',
        justifyContent: 'flex-start',
        position: 'relative',
        overflow: 'hidden',
      }}
      style={{
        backgroundColor: currentColors.sidebarBg,
      }}
    >
      {/* 首页/Logo 项 - 已更新尺寸、宽高比和悬停效果 */}
      <MuiIconButton
        onClick={() => handleItemClick({ name: 'home', path: '/home' })}
        sx={{
          marginTop: muiTheme.spacing(1.5),
          marginBottom: muiTheme.spacing(3),
          width: '55px',
          height: '48px', // 适应 Logo 高度并提供少许垂直边距
          padding: '0px',
          backgroundColor: 'transparent',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          transition: muiTheme.transitions.create(['transform', 'background-color', 'box-shadow'], { 
            duration: muiTheme.transitions.duration.shorter 
          }),
          '&:hover': {
            backgroundColor: 'transparent',
            boxShadow: 'none',
            transform: 'scale(1.1)', // Logo 悬停放大效果
          },
        }}
      >
        <img src={currentLogo} alt="ElfRadio 首页" style={{ width: '55px', height: 'auto', display: 'block' }} />
      </MuiIconButton>

      {/* 功能导航项容器 */}
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          width: '100%',
          flexGrow: 1,
          overflow: 'hidden',
          marginTop: muiTheme.spacing(2),
        }}
      >
        {navItems.map((item) => (
          <NavItemIcon
            key={item.name}
            icon={item.icon}
            tooltipTitle={item.tooltip}
            isSelected={selectedItem === item.name}
            onClick={() => handleItemClick(item)}
            currentColors={currentColors}
          />
        ))}
      </Box>

      {/* 主题切换项 */}
      <Box 
        sx={{ 
          position: 'absolute',
          bottom: muiTheme.spacing(2),
          left: '50%',
          transform: 'translateX(-50%)',
        }}
      >
        <Tooltip title={themeMode === 'dark' ? '切换到亮色模式' : '切换到暗色模式'} placement="right">
          <MuiIconButton
            onClick={toggleTheme}
            sx={{
              width: '48px',
              height: '48px',
              padding: '12px',
              borderRadius: '50%',
              backgroundColor: currentColors.innerDefaultFill,
              border: `1.5px solid ${currentColors.innerDefaultBorder}`,
              transition: muiTheme.transitions.create(['background-color', 'border-color', 'transform', 'color'], { 
                duration: muiTheme.transitions.duration.short, 
              }),
              '&:hover': {
                backgroundColor: currentColors.innerDefaultFill,
                borderColor: currentColors.innerHoverBorder,
                transform: 'scale(1.1)',
                '& .MuiSvgIcon-root': { 
                  color: currentColors.iconHover,
                },
              },
            }}
          >
            {themeMode === 'dark' 
              ? <LightModeRounded sx={{ fontSize: '24px', color: currentColors.iconDefault }} /> 
              : <DarkModeRounded sx={{ fontSize: '24px', color: currentColors.iconDefault }} />
            }
          </MuiIconButton>
        </Tooltip>
      </Box>
    </Box>
  );
};

export default Sidebar; 