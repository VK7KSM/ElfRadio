import React, { useState, useRef, useEffect } from 'react';
import {
  Box,
  Typography,
  Toolbar,
  Divider,
  useTheme,
  Theme,
} from '@mui/material';
import { SxProps } from '@mui/material/styles';

import StatusIndicatorDot from './StatusIndicatorDot'; // 步骤 5.1 创建的组件

// 导入 store hook 和正确的类型
// 假设 TaskStatus 是从 taskStore 导出的类型
import { useWebsocketStore } from '../lib/store/websocketStore';
import { useBackendStatusStore } from '../lib/store/backendStatusStore';
import { useTaskStore } from '../lib/store/taskStore'; //确保 TaskInfo 也导入
import { useSystemStatusStore } from '../lib/store/systemStatusStore'; // 确认 SystemServiceStatus 已导入
import { truncateUuidToFixedShortFormat } from '../lib/utils/stringUtils'; // 导入新的工具函数

// 如果 store 文件不导出完整的 state 类型，我们可能需要在这里定义它们
// 为了与提供的占位符代码一致，我们先定义简化的 store 状态类型
// 在实际集成（步骤 5.7）时，应确保从 store 获取正确的类型或在此处正确定义它们。
type WebSocketStatusValue = 'Connecting' | 'Connected' | 'Disconnected' | 'Error';
interface WebSocketStateForStatus {
  status: WebSocketStatusValue;
}

type BackendStatusValue = 'OK' | 'Error' | 'Checking' | 'Unknown';
interface BackendStateForStatus {
  status: BackendStatusValue;
}

// TaskStatus 类型，如果 useTaskStore 导出，则使用它
// type TaskStatus = 'Idle' | 'Running' | 'Stopping' | 'Error'; // 示例 - 现在将从 store 导入

const StatusBar: React.FC = () => {
  const theme = useTheme();

  // 2. 版本信息常量和 State (验证/定义)
  const VERSION_INFO_CORE_TEXT = "ElfRadio V0.1.0 (Dev) | © 2025 VK7KSM";
  const VERSION_INFO_LEADING_SPACES_STRING = " ".repeat(15); // 15 个常规空格
  const versionInfoDisplayFull = `${VERSION_INFO_LEADING_SPACES_STRING}${VERSION_INFO_CORE_TEXT}`;
  const versionInfoDisplayEllipsis = `...${VERSION_INFO_LEADING_SPACES_STRING}${VERSION_INFO_CORE_TEXT}`;

  const [currentVersionInfo, setCurrentVersionInfo] = useState(versionInfoDisplayFull);

  // 3. 定义硬编码的估算宽度常量 - 已移除相关常量
  // const ESTIMATED_VERSION_CORE_TEXT_WIDTH_PX = 200; // "ElfRadio V0.1.0..." 的估算值 - 已移除
  // const ESTIMATED_LEADING_SPACES_WIDTH_PX = 75;  // 15 个空格的估算值 (例如 15 * 5px/空格) - 已移除
  // const ESTIMATED_ELLIPSIS_WIDTH_PX = 15; // "..." 的宽度 (如果需要单独计算) - 已移除

  // 所有左侧项目 (Task, 短UUID, Radio...BK, 和它们之间的10个分隔符) 的估算总宽度
  // 示例：Task(180) + UUID_short(100) + Radio(60) + SDR(60) + LLM(60) + STT(60) + TTS(60) + Translate(80) + Online(70) + WS(60) + BK(60) = 850
  // 分隔符：10 * (大约每个16px，考虑边距) = 160
  // const ESTIMATED_LEFT_ITEMS_TOTAL_WIDTH_PX = 850 + 160; // 大约 1010px - 已移除

  // 4. Toolbar 引用
  const toolbarRef = useRef<HTMLDivElement>(null);

  // 2. Placeholder Data Definition - 移除 Task 相关占位符
  // const taskLabel = "Task:"; // 移除
  // const currentTaskName = "Run 模拟呼叫练习"; // 移除

  // --- Task Item Logic ---
  const taskCurrentStatusFromStore = useTaskStore((state) => state.status);
  const taskActiveInfoFromStore = useTaskStore((state) => state.activeTaskInfo); // This is TaskInfo | null

  let displayTaskName: string = "Idle";
  // Define dot status type based on actual needs, 'neutral' for Idle.
  let taskDotStatusForIndicator: 'ok' | 'warning' | 'neutral' = 'neutral';
  let taskNameTextColor: string = theme.palette.text.secondary;

  if (taskCurrentStatusFromStore === 'Running' && taskActiveInfoFromStore) {
      // taskActiveInfoFromStore is confirmed to be TaskInfo here
      const modeString = typeof taskActiveInfoFromStore.mode === 'string'
          ? taskActiveInfoFromStore.mode
          : JSON.stringify(taskActiveInfoFromStore.mode); // Fallback for non-string mode
      const nameString = taskActiveInfoFromStore.name || modeString; // Use name, fallback to mode
      displayTaskName = `Run ${nameString}`;
      taskDotStatusForIndicator = 'ok';
      taskNameTextColor = theme.palette.mode === 'dark' ? theme.palette.success.light : theme.palette.success.dark;
  } else if (taskCurrentStatusFromStore === 'Stopping') {
      displayTaskName = "Stopping...";
      taskDotStatusForIndicator = 'warning';
      taskNameTextColor = theme.palette.mode === 'dark' ? theme.palette.warning.light : theme.palette.warning.dark;
  } else if (taskCurrentStatusFromStore === 'Idle') {
      displayTaskName = "Idle 暂无任务运行";
      taskDotStatusForIndicator = 'neutral';
      taskNameTextColor = theme.palette.text.secondary;
  }
  // No 'Error' case here as TaskStatus from taskStore.ts does not include 'Error'.
  // If an error state needs to be represented for tasks, TaskStatus type would need an 'Error' variant.
  // For any other unexpected taskCurrentStatusFromStore value, it will fall into the 'Idle' default or the last explicit condition.

  // --- UUID Item Logic ---
  const userUuidFromStore = useSystemStatusStore((state) => state.userUuid); // userUuid is string | null
  const displayUserUuid = userUuidFromStore
      ? truncateUuidToFixedShortFormat(userUuidFromStore)
      : "N/A";
  const fullUserUuidTitle = userUuidFromStore || "UUID Not Available"; // For the tooltip

  // UUID (No dot, just label and value)
  // const uuidLabel = "UUID:";
  // const userUUID = "b187148a-2fd8-4cd8-a5cc-61c1806bb97d"; // Placeholder

  // Radio Item Logic
  const radioStatusFromStore = useSystemStatusStore((state) => state.radioStatus); // radioStatus is ConnectionStatus
  let radioDotForIndicator: 'ok' | 'warning' | 'error' | 'neutral' = 'neutral';
  if (radioStatusFromStore === 'Connected') {
      radioDotForIndicator = 'ok';
  } else if (radioStatusFromStore === 'Checking') {
      radioDotForIndicator = 'warning';
  } else if (radioStatusFromStore === 'Disconnected' || radioStatusFromStore === 'Error') {
      radioDotForIndicator = 'error';
  } else if (radioStatusFromStore === 'Unknown') {
      radioDotForIndicator = 'neutral';
  }

  // SDR Item Logic
  const sdrStatusFromStore = useSystemStatusStore((state) => state.sdrStatus); // sdrStatus is ConnectionStatus
  let sdrDotForIndicator: 'ok' | 'warning' | 'error' | 'neutral' = 'neutral';
  if (sdrStatusFromStore === 'Connected') {
      sdrDotForIndicator = 'ok';
  } else if (sdrStatusFromStore === 'Checking') {
      sdrDotForIndicator = 'warning';
  } else if (sdrStatusFromStore === 'Disconnected' || sdrStatusFromStore === 'Error') {
      // SDR is a placeholder, its store default is 'Disconnected', so it will show 'error' (red dot)
      sdrDotForIndicator = 'error';
  } else if (sdrStatusFromStore === 'Unknown') {
      sdrDotForIndicator = 'neutral';
  }
  
  // --- LLM Item Logic ---
  const llmStatusFromStore = useSystemStatusStore((state) => state.llmStatus); // llmStatus is SystemServiceStatus
  let llmDotForIndicator: 'ok' | 'warning' | 'error' | 'neutral' = 'neutral'; // Default to neutral if unknown
  if (llmStatusFromStore === 'Ok') {
      llmDotForIndicator = 'ok';
  } else if (llmStatusFromStore === 'Warning') {
      llmDotForIndicator = 'warning';
  } else if (llmStatusFromStore === 'Error') {
      llmDotForIndicator = 'error';
  } else if (llmStatusFromStore === 'Unknown') {
      llmDotForIndicator = 'neutral'; // Or 'disabled' if you prefer a grey dot for Unknown
  }

  // STT Item Logic
  const sttStatusFromStore = useSystemStatusStore((state) => state.sttStatus); // sttStatus is SystemServiceStatus
  let sttDotForIndicator: 'ok' | 'warning' | 'error' | 'neutral' = 'neutral';
  if (sttStatusFromStore === 'Ok') {
      sttDotForIndicator = 'ok';
  } else if (sttStatusFromStore === 'Warning') {
      sttDotForIndicator = 'warning';
  } else if (sttStatusFromStore === 'Error') {
      sttDotForIndicator = 'error';
  } else if (sttStatusFromStore === 'Unknown') {
      sttDotForIndicator = 'neutral';
  }

  // TTS Item Logic
  const ttsStatusFromStore = useSystemStatusStore((state) => state.ttsStatus); // ttsStatus is SystemServiceStatus
  let ttsDotForIndicator: 'ok' | 'warning' | 'error' | 'neutral' = 'neutral';
  if (ttsStatusFromStore === 'Ok') {
      ttsDotForIndicator = 'ok';
  } else if (ttsStatusFromStore === 'Warning') {
      ttsDotForIndicator = 'warning';
  } else if (ttsStatusFromStore === 'Error') {
      ttsDotForIndicator = 'error';
  } else if (ttsStatusFromStore === 'Unknown') {
      ttsDotForIndicator = 'neutral';
  }

  // Translate Item Logic
  const translateStatusFromStore = useSystemStatusStore((state) => state.translateStatus); // translateStatus is SystemServiceStatus
  let translateDotForIndicator: 'ok' | 'warning' | 'error' | 'neutral' = 'neutral';
  if (translateStatusFromStore === 'Ok') {
      translateDotForIndicator = 'ok';
  } else if (translateStatusFromStore === 'Warning') {
      translateDotForIndicator = 'warning';
  } else if (translateStatusFromStore === 'Error') {
      translateDotForIndicator = 'error';
  } else if (translateStatusFromStore === 'Unknown') {
      translateDotForIndicator = 'neutral';
  }

  // Online (Network) Item Logic
  const networkStatusFromStore = useSystemStatusStore((state) => state.networkStatus); // networkStatus is ConnectionStatus
  // --- BEGIN ADDED DEBUG LOG for StatusBar.tsx ---
  console.log('[StatusBar.tsx DEBUG] networkStatusFromStore:', networkStatusFromStore, 'at', new Date().toLocaleTimeString());
  // --- END ADDED DEBUG LOG for StatusBar.tsx ---

  let onlineDotForIndicator: 'ok' | 'warning' | 'error' | 'neutral' = 'neutral';
  if (networkStatusFromStore === 'Connected') {
      onlineDotForIndicator = 'ok';
  } else if (networkStatusFromStore === 'Checking') {
      onlineDotForIndicator = 'warning';
  } else if (networkStatusFromStore === 'Disconnected' || networkStatusFromStore === 'Error') {
      onlineDotForIndicator = 'error';
  } else if (networkStatusFromStore === 'Unknown') {
      onlineDotForIndicator = 'neutral';
  }

  // Placeholder Data - 移除 LLM 相关占位符
  // const llmLabel = "LLM:"; // 移除
  // const llmDotStatus = 'warning'; // 移除

  // Other Status Items (Label + Dot only)
  // const radioLabel = "Radio:"; const radioDotStatus = 'error'; // 已移除
  // const sdrLabel = "SDR:"; const sdrDotStatus = 'error'; // 已移除
  // const sttLabel = "STT:"; const sttDotStatus = 'warning'; // 已移除
  // const ttsLabel = "TTS:"; const ttsDotStatus = 'warning'; // 已移除
  // const translateLabel = "Translate:"; const translateDotStatus = 'warning'; // 已移除
  // const onlineLabel = "Online:"; const onlineDotStatus = 'ok'; // 已移除

  // Existing Status Items (to be adapted)
  const wsStatusFromStore = useWebsocketStore((state: WebSocketStateForStatus) => state.status);
  const wsLabel = "WS:";
  const wsDotStatus = wsStatusFromStore === 'Connected' ? 'ok' : (wsStatusFromStore === 'Connecting' ? 'warning' : 'error');

  const backendStatusFromStore = useBackendStatusStore((state: BackendStateForStatus) => state.status);
  const bkLabel = "BK:";
  const bkDotStatus = backendStatusFromStore === 'OK' ? 'ok' : (backendStatusFromStore === 'Checking' ? 'warning' : 'error');
  
  const statusItemSx: SxProps<Theme> = {
    display: 'flex',
    alignItems: 'center',
    whiteSpace: 'nowrap',
  };

  const labelSx: SxProps<Theme> = {
    color: theme.palette.text.secondary,
    mr: 0.5,
  };

  const dividerSx: SxProps<Theme> = {
    height: '16px',
    ml: 1,
    mr: 1,
    borderColor: theme.palette.divider,
  };

  // 5. 实现 useEffect 与 ResizeObserver
  useEffect(() => {
    const toolbarElement = toolbarRef.current;
    if (!toolbarElement) return;

    const resizeObserver = new ResizeObserver(entries => {
      for (let entry of entries) {
        if (entry.target === toolbarElement) {
          const totalToolbarWidth = entry.contentRect.width;

          // if (totalToolbarWidth < (ESTIMATED_LEFT_ITEMS_TOTAL_WIDTH_PX + ESTIMATED_LEADING_SPACES_WIDTH_PX)) {
          if (totalToolbarWidth < 1099) { // 使用新的精确阈值
            setCurrentVersionInfo(prev => (prev !== versionInfoDisplayEllipsis ? versionInfoDisplayEllipsis : prev));
          } else {
            setCurrentVersionInfo(prev => (prev !== versionInfoDisplayFull ? versionInfoDisplayFull : prev));
          }
        }
      }
    });
    resizeObserver.observe(toolbarElement);
    return () => {
      if (toolbarElement) {
        resizeObserver.unobserve(toolbarElement);
      }
      resizeObserver.disconnect(); // 确保调用 disconnect
    };
  }, [versionInfoDisplayFull, versionInfoDisplayEllipsis]); // 依赖项是稳定的

  return (
    <Box component="footer" sx={{ width: '100%'}}>
      <Toolbar 
        ref={toolbarRef}
        variant="dense" 
        sx={{ 
          justifyContent: 'space-between', 
          minHeight: '32px', 
          alignItems: 'center', 
          width: '100%', 
          px: 0, 
          m: 0,
          display: 'flex', 
          overflow: 'hidden', 
        }}
      >
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            flexGrow: 1,       
            flexShrink: 1,     
            minWidth: 0,       
            overflow: 'hidden',  // CRITICAL: 添加 overflow: 'hidden'
          }}
        >
          {/* Left-aligned items container */}
          <Box sx={statusItemSx}>
            <Typography variant="caption" sx={labelSx}>
              Task:
            </Typography>
            <StatusIndicatorDot status={taskDotStatusForIndicator} />
            <Typography variant="caption" sx={{ ml: 0.5, color: taskNameTextColor }}>
              {displayTaskName}
            </Typography>
          </Box>
          <Divider orientation="vertical" flexItem sx={dividerSx} />

          {/* UUID Item Box */}
          <Box sx={statusItemSx}>
            <Typography variant="caption" sx={labelSx}>
              UUID:
            </Typography>
            <Typography
              variant="caption"
              sx={{ color: theme.palette.text.primary }}
              title={fullUserUuidTitle} // Use the full UUID for the tooltip
            >
              {displayUserUuid} {/* Use the dynamically prepared display string */}
            </Typography>
          </Box>
          <Divider orientation="vertical" flexItem sx={dividerSx} />

          {/* Radio Item Box */}
          <Box sx={statusItemSx}>
            <Typography variant="caption" sx={labelSx}>
              Radio:
            </Typography>
            <StatusIndicatorDot status={radioDotForIndicator} />
          </Box>
          <Divider orientation="vertical" flexItem sx={dividerSx} />

          {/* SDR Item Box */}
          <Box sx={statusItemSx}>
            <Typography variant="caption" sx={labelSx}>
              SDR:
            </Typography>
            <StatusIndicatorDot status={sdrDotForIndicator} />
          </Box>
          <Divider orientation="vertical" flexItem sx={dividerSx} />

          {/* LLM Item Box */}
          <Box sx={statusItemSx}>
            <Typography variant="caption" sx={labelSx}>
              LLM:
            </Typography>
            <StatusIndicatorDot status={llmDotForIndicator} />
          </Box>
          <Divider orientation="vertical" flexItem sx={dividerSx} />

          {/* STT Item Box */}
          <Box sx={statusItemSx}>
            <Typography variant="caption" sx={labelSx}>
              STT:
            </Typography>
            <StatusIndicatorDot status={sttDotForIndicator} />
          </Box>
          <Divider orientation="vertical" flexItem sx={dividerSx} />

          {/* TTS Item Box */}
          <Box sx={statusItemSx}>
            <Typography variant="caption" sx={labelSx}>
              TTS:
            </Typography>
            <StatusIndicatorDot status={ttsDotForIndicator} />
          </Box>
          <Divider orientation="vertical" flexItem sx={dividerSx} />

          {/* Translate Item Box */}
          <Box sx={statusItemSx}>
            <Typography variant="caption" sx={labelSx}>
              Translate:
            </Typography>
            <StatusIndicatorDot status={translateDotForIndicator} />
          </Box>
          <Divider orientation="vertical" flexItem sx={dividerSx} />
          
          {/* Online Item Box (as per prompt, assuming this means general online/cloud connectivity, not WS specific) */}
          <Box sx={statusItemSx}>
            <Typography variant="caption" sx={labelSx}>
              Online:
            </Typography>
            <StatusIndicatorDot status={onlineDotForIndicator} />
          </Box>
          <Divider orientation="vertical" flexItem sx={dividerSx} />

          {/* WS Item Box */}
          <Box sx={statusItemSx}>
            <Typography variant="caption" sx={labelSx}>
              {wsLabel}
            </Typography>
            <StatusIndicatorDot status={wsDotStatus} />
          </Box>
          <Divider orientation="vertical" flexItem sx={dividerSx} />

          {/* BK Item Box */}
          <Box sx={statusItemSx}>
            <Typography variant="caption" sx={labelSx}>
              {bkLabel}
            </Typography>
            <StatusIndicatorDot status={bkDotStatus} />
          </Box>
          {/* No divider after the last item in the left group */}
        </Box>

        {/* Version Info (pushed to the right) */}
        <Typography
          variant="caption"
          sx={{
            whiteSpace: 'pre', // CRITICAL: 更改为 'pre'
            color: theme.palette.text.secondary,
            flexShrink: 0,    // 保持: 防止此 Typography 收缩
          }}
        >
          {currentVersionInfo} {/* 使用 state 变量 */}
        </Typography>
      </Toolbar>
    </Box>
  );
};

export default StatusBar; 