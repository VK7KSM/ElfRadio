import React from 'react';
import { Box, useTheme, Theme } from '@mui/material';
import { SxProps } from '@mui/material/styles';

interface StatusIndicatorDotProps {
  status: 'ok' | 'success' | 'error' | 'warning' | 'neutral' | 'disabled' | string;
}

const StatusIndicatorDot: React.FC<StatusIndicatorDotProps> = ({ status }) => {
  const theme = useTheme();

  const getColor = (): string => {
    switch (status) {
      case 'ok':
      case 'success':
        return theme.palette.success.main;
      case 'error':
        return theme.palette.error.main;
      case 'warning':
        // return '#FFEB3B'; // 之前的亮黄色，在浅蓝色背景下对比度不够
        return '#F9A825'; // 更深的黄色 (Material Yellow 800)，更高对比度
      case 'neutral':
      case 'disabled':
        return theme.palette.grey[400];
      default:
        // 尝试将 status 视为直接的 CSS 颜色值
        if (status.startsWith('#') || status.startsWith('rgb') || status.startsWith('hsl')) {
          return status;
        }
        return theme.palette.grey[400];
    }
  };

  const dotSx: SxProps<Theme> = {
    width: '11px',
    height: '11px',
    borderRadius: '50%',
    backgroundColor: getColor(),
    display: 'inline-block',
    verticalAlign: 'middle',
    position: 'relative', 
    top: '-2px', // 向上调整更多，使其与文字的垂直中心对齐
  };

  return <Box component="span" sx={dotSx} />;
};

export default StatusIndicatorDot; 