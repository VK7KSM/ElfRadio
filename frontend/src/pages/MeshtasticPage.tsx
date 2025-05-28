import React from 'react';
import Typography from '@mui/material/Typography';
import Box from '@mui/material/Box';

/**
 * Meshtastic 页面
 * 占位页面，用于路由测试
 */
const MeshtasticPage: React.FC = () => {
  return (
    <Box sx={{ p: 3, height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
      <Typography variant="h4">Meshtastic 页面 (占位)</Typography>
    </Box>
  );
};

export default MeshtasticPage; 