import React, { useEffect, useRef } from 'react';
import { Box, Typography, Paper, IconButton, Switch, FormControlLabel } from '@mui/material';
import DeleteSweepIcon from '@mui/icons-material/DeleteSweep';
import { useLogStore } from '../lib/store/logStore'; // Adjust path if necessary

const LogDisplay: React.FC = () => {
  const messages = useLogStore((state) => state.messages);
  const clearLogs = useLogStore((state) => state.clearLogs);
  const logContainerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = React.useState(true);

  // Effect to scroll to bottom when new messages arrive if autoScroll is enabled
  useEffect(() => {
    if (autoScroll && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [messages, autoScroll]); // Rerun effect when messages or autoScroll changes

  return (
    <Paper elevation={2} sx={{ display: 'flex', flexDirection: 'column', height: '300px', // Example fixed height, adjust as needed
                                 overflow: 'hidden', p: 1 }}>
       <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', pb: 1, borderBottom: 1, borderColor: 'divider', flexShrink: 0 /* Prevent header shrinking */ }}>
            <Typography variant="overline">应用程序日志</Typography>
            <Box>
                <FormControlLabel
                    control={<Switch checked={autoScroll} onChange={(e) => setAutoScroll(e.target.checked)} size="small" />}
                    label={<Typography variant="caption">自动滚动</Typography>}
                    sx={{mr: 1}}
                />
                <IconButton size="small" onClick={clearLogs} title="清除日志">
                    <DeleteSweepIcon fontSize="small" />
                </IconButton>
            </Box>
       </Box>
       <Box
         ref={logContainerRef}
         sx={{
           flexGrow: 1,
           overflowY: 'auto', // Enable vertical scrolling
           fontFamily: 'monospace',
           fontSize: 'caption.fontSize',
           whiteSpace: 'pre-wrap', // Keep line breaks from the source
           wordBreak: 'break-all', // Prevent long unbroken strings from overflowing
           py: 1, // Padding top/bottom inside the log area
         }}
       >
         {messages.map((msg, index) => (
           // Using index as key is acceptable here because logs are append-only
           // and we don't reorder/delete individual items other than from the start.
           <div key={index}>{msg}</div>
         ))}
         {messages.length === 0 && (
            <Typography variant="caption" sx={{ color: 'text.secondary', fontStyle: 'italic', display: 'block', textAlign: 'center', mt: 2 }}>
                暂无日志...
            </Typography>
         )}
       </Box>
    </Paper>
  );
};

export default LogDisplay; 