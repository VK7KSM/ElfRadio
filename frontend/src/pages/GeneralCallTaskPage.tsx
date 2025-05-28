import React, { useState, useEffect } from 'react';
import { Box, Typography, Button, Switch, FormControlLabel, useTheme, Paper, Select, MenuItem, FormControl, Avatar, Grid, Tooltip, IconButton, Divider, Stack, TextField, Chip, List, ListItem, ListItemButton, InputAdornment, Collapse } from '@mui/material';
import { alpha } from '@mui/material/styles'; 
import PlayArrowRoundedIcon from '@mui/icons-material/PlayArrowRounded';
import PauseRoundedIcon from '@mui/icons-material/PauseRounded';
import StopRoundedIcon from '@mui/icons-material/StopRounded'; 
import AutoAwesomeOutlinedIcon from '@mui/icons-material/AutoAwesomeOutlined';
import SaveAltIcon from '@mui/icons-material/SaveAlt';
import ReplyIcon from '@mui/icons-material/Reply';
import CancelIcon from '@mui/icons-material/Cancel';
import MicIcon from '@mui/icons-material/Mic';
import GraphicEqIcon from '@mui/icons-material/GraphicEq';
import SendIcon from '@mui/icons-material/Send';
import TranslateIcon from '@mui/icons-material/Translate';
import VolumeUpIcon from '@mui/icons-material/VolumeUp';
import KeyboardVoiceIcon from '@mui/icons-material/KeyboardVoice';
import ImageIcon from '@mui/icons-material/Image';
import KeyboardIcon from '@mui/icons-material/Keyboard';
import SearchIcon from '@mui/icons-material/Search';
import AddCircleOutlineIcon from '@mui/icons-material/AddCircleOutline';
import KeyboardArrowUpIcon from '@mui/icons-material/KeyboardArrowUp';
import KeyboardArrowDownIcon from '@mui/icons-material/KeyboardArrowDown';

// I. Import Necessary Assets
import sstvPlaceholderImage from '../assets/logo200.png';
import contactAvatar000 from '../assets/contacts/000.jpg';
import contactAvatar001 from '../assets/contacts/001.jpg';
import contactAvatar002 from '../assets/contacts/002.jpg';
import unknownAvatarForMsg from '../assets/unknown/avatar_01.jpg';

// I. Define Message Data Structures (Placeholder)
// 模拟消息数据结构 (后续会从 chatStore 或 API 获取)
interface BaseMessage {
  id: string;
  sender: string; // 呼号或姓名
  timestamp: string; // 格式: YYYY/MM/DD HH:MM:SS 或 YYYY/MM/DD --:--:--
  receivedTimestamp?: string; // 仅接收消息
  status?: 'Queued' | 'Sending' | 'Sent' | 'Cancelled' | 'Received'; // 发送状态或接收状态
  messageType: 'Voice' | 'CW Text' | 'SSTV' | 'Unknown';
  isSender: boolean; // true 如果是当前用户发送的
}

interface AudioMessageContent {
  audioUrl?: string; // 播放器使用
  duration?: string; // 例如 "00:27"
  originalText?: string; // 原始输入 (TTS)
  sttText?: string;
  sttLang?: 'EN' | 'CN' | 'FR' | 'JP';
  translatedText?: string;
  translatedLang?: 'EN' | 'CN' | 'FR' | 'JP';
}

interface CwMessageContent {
  dotsAndDashes?: string; // 点划
  decodedText?: string; // CW识别后的文字
}

interface SstvMessageContent {
  imageUrl?: string; // 预览图片URL
  imageDimensions?: string; // 例如 "300x256"
}

interface VoiceMessage extends BaseMessage, AudioMessageContent {
  messageType: 'Voice';
}

interface CwTextMessage extends BaseMessage, AudioMessageContent, CwMessageContent { // CW 也可能有音频和STT/TT
  messageType: 'CW Text';
}

interface SstvPicMessage extends BaseMessage, SstvMessageContent {
  messageType: 'SSTV';
}

interface UnknownContentMessage extends BaseMessage {
   messageType: 'Unknown';
   rawSignalData?: string; // 示例原始信号数据
}

type Message = VoiceMessage | CwTextMessage | SstvPicMessage | UnknownContentMessage;

// II. Update Placeholder Message Data Structures (`mockMessages`)
const mockMessages: Message[] = [
  // 1. Sent - Instant audio recording (Voice)
  {
    id: 'sent-audio-1',
    sender: 'You',
    timestamp: '2025/05/16 09:30:00',
    status: 'Sent',
    messageType: 'Voice',
    isSender: true,
    audioUrl: '#placeholder-audio-url',
    duration: '00:12',
    sttText: '这是即时录音的识别文本',
    sttLang: 'CN',
  },
  // 2. Sent - Text-to-human speech (Voice)
  {
    id: 'sent-tts-1',
    sender: 'You',
    timestamp: '2025/05/16 09:31:00',
    status: 'Sent',
    messageType: 'Voice',
    isSender: true,
    originalText: '你好，世界！这是一条TTS语音消息。',
    duration: '00:08',
    audioUrl: '#placeholder-tts-audio-url',
  },
  // 3. Sent - Text-to-CW audio (CW Text)
  {
    id: 'sent-text-to-cw-1',
    sender: 'You',
    timestamp: '2025/05/16 09:32:00',
    status: 'Sending', // 演示发送中状态
    messageType: 'CW Text',
    isSender: true,
    originalText: 'TEST CW MSG',
    dotsAndDashes: '- . ... - / -.-. .-- / -- ... --.',
    decodedText: 'TEST CW MSG',
    duration: '00:10',
    audioUrl: '#placeholder-cw-audio-url',
  },
  // 4. Sent - Image SSTV (SSTV, Martin1)
  {
    id: 'sent-sstv-1',
    sender: 'You',
    timestamp: '2025/05/16 09:33:00',
    status: 'Queued', // 演示排队中状态
    messageType: 'SSTV',
    isSender: true,
    imageUrl: sstvPlaceholderImage, // 使用导入的占位符
    imageDimensions: '200x200', // 假设logo200是这个尺寸
  },
  // 5. Received - Voice audio (Voice)
  {
    id: 'received-audio-1',
    sender: 'VK7KSM',
    timestamp: '2025/05/16 09:35:00',
    receivedTimestamp: '2025/05/16 09:35:05',
    status: 'Received',
    messageType: 'Voice',
    isSender: false,
    audioUrl: '#placeholder-received-audio-url',
    duration: '00:27',
    sttText: '(EN) This is a received voice message.',
    sttLang: 'EN',
    translatedText: '(CN) 这是一条收到的语音消息。',
    translatedLang: 'CN',
  },
  // 6. Received - CW audio (CW Text)
  {
    id: 'received-cw-1',
    sender: 'JA1ABC',
    timestamp: '2025/05/16 09:36:00',
    receivedTimestamp: '2025/05/16 09:36:08',
    status: 'Received',
    messageType: 'CW Text',
    isSender: false,
    audioUrl: '#placeholder-received-cw-audio-url',
    duration: '00:22',
    dotsAndDashes: '.... . .-.. .-.. --- / .-- --- .-. .-.. -..',
    decodedText: 'HELLO WORLD',
    sttText: 'HELLO WORLD', // 假设CW解码后也进行了STT
    sttLang: 'EN',
  },
  // 7. Received - Image SSTV (SSTV, Martin1)
  {
    id: 'received-sstv-1',
    sender: 'DL2XYZ',
    timestamp: '2025/05/16 09:37:00',
    receivedTimestamp: '2025/05/16 09:38:10', // 较长接收时间
    status: 'Received',
    messageType: 'SSTV',
    isSender: false,
    imageUrl: sstvPlaceholderImage,
    imageDimensions: '200x200',
  },
  // 8. Received - Unknown signal (Unknown)
  {
    id: 'received-unknown-1',
    sender: 'N0CALL',
    timestamp: '2025/05/16 09:39:00',
    receivedTimestamp: '2025/05/16 09:39:02',
    status: 'Received',
    messageType: 'Unknown',
    isSender: false,
    rawSignalData: '无法识别的信号数据片段...强度-75dBm...',
  },
];

// IV.B. MessageBubble Component Refinements
interface MessageBubbleProps {
  message: Message;
}

// 模拟已知联系人列表，用于MessageBubble判断头像
const knownContactsCallsigns = ['VK7KSM', 'JA1ABC', 'DL2XYZ']; 
// 模拟已知联系人头像映射
const contactAvatarsMap: { [key: string]: string } = {
  'VK7KSM': contactAvatar000,
  'JA1ABC': contactAvatar001, // JA1ABC 使用原 N0CALL 的头像 (contactAvatar001)
  'DL2XYZ': contactAvatar002,
};

const MessageBubble: React.FC<MessageBubbleProps> = ({ message }) => {
  const theme = useTheme();
  const isSender = message.isSender;

  const bubbleAlignment = isSender ? 'flex-end' : 'flex-start';
  
  // 使用Material Design 3配色图中的颜色，确保有良好的辨识度
  // 发送消息：使用Tertiary Container（浅紫色）
  // 接收消息：使用Primary Container（浅青色）
  const sentBubbleBgColor = theme.palette.mode === 'dark' ? '#4f378b' : '#e8def8';
  const sentBubbleTextColor = theme.palette.mode === 'dark' ? '#e8def8' : '#1d192b';
  
  const receivedBubbleBgColor = theme.palette.mode === 'dark' ? '#0f4c75' : '#cce7ff';
  const receivedBubbleTextColor = theme.palette.mode === 'dark' ? '#cce7ff' : '#001d36';
  
  const bubbleBgColor = isSender ? sentBubbleBgColor : receivedBubbleBgColor;
  const bubbleTextColor = isSender ? sentBubbleTextColor : receivedBubbleTextColor;

  // 音频播放器组件 - 根据发送/接收使用不同颜色
  const AudioPlayer = ({ duration }: { duration: string }) => {
    // 为发送消息使用紫色调，为接收消息使用蓝色调
    const playerBgColor = isSender 
      ? alpha(theme.palette.secondary.main, 0.2) // 紫色调
      : alpha(theme.palette.primary.main, 0.2);  // 蓝色调
    
    const playerIconColor = isSender 
      ? theme.palette.secondary.main 
      : theme.palette.primary.main;

    return (
      <Box sx={{
        display: 'flex',
        alignItems: 'center',
        bgcolor: playerBgColor,
        borderRadius: '14px',
        p: theme.spacing(0.5, 1),
        my: 1,
        minHeight: '28px'
      }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, flexGrow: 1 }}>
          <IconButton size="small" sx={{ 
            color: playerIconColor,
            padding: '2px',
          }}>
            <PlayArrowRoundedIcon fontSize="small" />
          </IconButton>
          <Box sx={{ 
            flexGrow: 1,
            height: '2px', 
            bgcolor: theme.palette.action.disabled,
            borderRadius: '1px',
            mx: 1
          }} />
          <Typography variant="caption" sx={{ 
            color: bubbleTextColor,
            fontSize: '0.7rem'
          }}>
            {duration}
          </Typography>
        </Box>
      </Box>
    );
  };

  return (
    <Box sx={{ 
      display: 'flex', 
      justifyContent: bubbleAlignment, 
      width: '100%',
      mb: 2
    }}>
      {!isSender && (
        <Avatar 
          src={knownContactsCallsigns.includes(message.sender) ? contactAvatarsMap[message.sender] : unknownAvatarForMsg}
          sx={{ 
            width: 32,  // 消息流中头像保持32x32
            height: 32, 
            mr: 1, 
            alignSelf: 'flex-start', 
            mt: 0.5, 
            // 对于字母头像，如果需要，可以添加背景色逻辑
            fontSize: '0.875rem', 
            fontWeight: 'bold',
            // 如果 src 解析失败或不存在，Avatar 会默认显示字母
            // bgcolor: knownContactsCallsigns.includes(message.sender) && !contactAvatarsMap[message.sender] ? theme.palette.primary.main : theme.palette.grey[600],
            // color: knownContactsCallsigns.includes(message.sender) && !contactAvatarsMap[message.sender] ? theme.palette.primary.contrastText : theme.palette.common.white,
          }}
        >
          {/* 如果 src 图片有效，则不显示字母；否则显示发送者首字母 */}
          {!(knownContactsCallsigns.includes(message.sender) ? contactAvatarsMap[message.sender] : unknownAvatarForMsg) && message.sender.substring(0, 1).toUpperCase()}
        </Avatar>
      )}
      
      <Paper
        elevation={1}
        sx={{
          borderRadius: '10px',
          width: '380px',
          bgcolor: bubbleBgColor,
          color: bubbleTextColor,
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden'
        }}
      >
        {/* Header Row - 修改发送消息的状态显示和取消按钮位置 */}
        <Box sx={{ 
          display: 'flex', 
          justifyContent: 'space-between', 
          alignItems: 'center', 
          p: theme.spacing(1, 1.5, 0.5, 1.5)
        }}>
          <Typography variant="caption" sx={{ 
            fontWeight: 'bold',
            fontSize: '0.8rem'
          }}>
            {isSender ? 'You' : message.sender}
          </Typography>
          
          {/* 右侧状态区域 */}
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
            <Typography variant="caption" sx={{ 
              fontSize: '0.75rem',
              fontWeight: 'medium'
            }}>
              {isSender ? message.status : message.messageType}
            </Typography>
            
            {/* 发送消息的取消按钮移到状态后面 */}
            {isSender && (message.status === 'Sending' || message.status === 'Queued') && (
              <Tooltip title="取消发送">
                <IconButton 
                  size="small" 
                  sx={{ 
                    color: theme.palette.warning.main,
                    width: 20,
                    height: 20,
                    ml: 0.5
                  }}
                >
                  <CancelIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            )}
          </Box>
        </Box>

        {/* Main Content Area - 根据消息类型显示不同内容 */}
        <Box sx={{ px: 1.5, pb: 0.5 }}>
          {/* 1. 发送 - 即时录音语音消息 (音频播放器 → STT → TT) */}
          {isSender && message.messageType === 'Voice' && message.id === 'sent-audio-1' && (
            <Box>
              {/* 音频播放器 */}
              {message.duration && <AudioPlayer duration={message.duration} />}
              
              {/* STT */}
              {message.sttText && (
                <Typography variant="body2" sx={{ 
                  fontSize: '0.875rem', 
                  mt: 1, 
                  mb: 0.5,
                  color: theme.palette.info.main // 系统生成的文字用青色
                }}>
                  STT : ({message.sttLang}) {message.sttText}
                </Typography>
              )}
              
              {/* TT - 这里需要模拟翻译文本，因为原数据中没有 */}
              <Typography variant="body2" sx={{ 
                fontSize: '0.875rem',
                mt: 0.5,
                color: theme.palette.info.main // 系统生成的文字用青色
              }}>
                TT : (EN) This is the recognized text from instant recording
              </Typography>
            </Box>
          )}

          {/* 2. 发送 - 文字转语音消息 (原始文字消息 → TT → 音频播放器) */}
          {isSender && message.messageType === 'Voice' && message.id === 'sent-tts-1' && (
            <Box>
              {/* 原始文字消息 - 用黑色（或气泡默认文字颜色） */}
              {message.originalText && (
                <Typography variant="body2" sx={{ 
                  fontSize: '0.875rem', 
                  mt: 1, 
                  mb: 1,
                  color: bubbleTextColor // 原始文字用默认颜色（黑色）
                }}>
                  {message.originalText}
                </Typography>
              )}
              
              {/* TT - 系统生成的文字用青色 */}
              <Typography variant="body2" sx={{ 
                fontSize: '0.875rem',
                mb: 1,
                color: theme.palette.info.main // 系统生成的文字用青色
              }}>
                TT : (EN) Hello, world! This is a TTS voice message.
              </Typography>
              
              {/* 音频播放器 */}
              {message.duration && <AudioPlayer duration={message.duration} />}
            </Box>
          )}

          {/* 3. 发送 - 文字转CW消息 (原始文字消息 → TT → CW → 音频播放器) */}
          {isSender && message.messageType === 'CW Text' && (
            <Box>
              {/* 原始文字消息 - 用黑色（或气泡默认文字颜色） */}
              {message.originalText && (
                <Typography variant="body2" sx={{ 
                  fontSize: '0.875rem', 
                  mt: 1, 
                  mb: 1,
                  color: bubbleTextColor // 原始文字用默认颜色（黑色）
                }}>
                  {message.originalText}
                </Typography>
              )}
              
              {/* TT - 系统生成的文字用青色 */}
              <Typography variant="body2" sx={{ 
                fontSize: '0.875rem',
                mb: 1,
                color: theme.palette.info.main // 系统生成的文字用青色
              }}>
                TT : (EN) The weather is good today
              </Typography>
              
              {/* CW - 系统生成的文字用青色，并确保字体一致 */}
              {message.dotsAndDashes && (
                <Typography variant="body2" sx={{ 
                  fontSize: '0.875rem', // 与其他文字字体大小一致
                  mb: 1,
                  color: theme.palette.info.main // 系统生成的文字用青色
                }}>
                  CW : {message.dotsAndDashes}
                </Typography>
              )}
              
              {/* 音频播放器 */}
              {message.duration && <AudioPlayer duration={message.duration} />}
            </Box>
          )}

          {/* 4. 发送 - 图片SSTV消息 (压缩后的图片 → 图片分辨率 → 音频播放器) */}
          {isSender && message.messageType === 'SSTV' && (
            <Box>
              {/* 压缩后的图片 */}
              <Box sx={{ 
                textAlign: 'center',
                bgcolor: alpha(theme.palette.common.white, 0.1),
                borderRadius: '8px',
                p: 1,
                mt: 1,
                mb: 1
              }}>
                {'imageUrl' in message && message.imageUrl && (
                  <img 
                    src={message.imageUrl} 
                    alt="SSTV 图片" 
                    style={{ 
                      width: '100%',
                      height: 'auto', 
                      maxHeight: '180px',
                      objectFit: 'contain', 
                      borderRadius: '6px'
                    }} 
                  />
                )}
              </Box>
              
              {/* 图片分辨率 */}
              {'imageDimensions' in message && message.imageDimensions && (
                <Typography variant="body2" sx={{ 
                  fontSize: '0.875rem',
                  textAlign: 'center',
                  mb: 1,
                  color: bubbleTextColor // 使用默认文字颜色
                }}>
                  分辨率 : {message.imageDimensions}
                </Typography>
              )}
              
              {/* 音频播放器 - SSTV图片也有音频传输 */}
              <AudioPlayer duration="01:18" />
            </Box>
          )}

          {/* 5. 接收 - 音频消息（人类语音）: 音频播放器 → STT → TT */}
          {!isSender && message.messageType === 'Voice' && (
            <Box>
              {/* 音频播放器 */}
              {message.duration && <AudioPlayer duration={message.duration} />}
              
              {/* STT */}
              {message.sttText && (
                <Typography variant="body2" sx={{ 
                  fontSize: '0.875rem', 
                  mt: 1, 
                  mb: 0.5,
                  color: bubbleTextColor
                }}>
                  STT : {message.sttText}
                </Typography>
              )}
              
              {/* TT */}
              {message.translatedText && (
                <Typography variant="body2" sx={{ 
                  fontSize: '0.875rem',
                  mt: 0.5,
                  color: theme.palette.info.main
                }}>
                  TT : {message.translatedText}
                </Typography>
              )}
            </Box>
          )}

          {/* 6. 接收 - 文字消息（CW 音频）: 音频播放器 → CW → Text → TT */}
          {!isSender && message.messageType === 'CW Text' && (
            <Box>
              {/* 音频播放器 */}
              {message.duration && <AudioPlayer duration={message.duration} />}
              
              {/* CW - 原始CW码用黑色 */}
              {message.dotsAndDashes && (
                <Typography variant="body2" sx={{ 
                  fontSize: '0.875rem',
                  mt: 1,
                  mb: 0.5,
                  color: bubbleTextColor // 保持黑色
                }}>
                  CW : {message.dotsAndDashes}
                </Typography>
              )}
              
              {/* Text - 解码文字改为蓝色 */}
              {message.decodedText && (
                <Typography variant="body2" sx={{ 
                  fontSize: '0.875rem', 
                  mt: 0.5, 
                  mb: 0.5,
                  color: theme.palette.info.main // 改为蓝色
                }}>
                  Text : {message.decodedText}
                </Typography>
              )}
              
              {/* TT - 翻译结果用蓝色 */}
              <Typography variant="body2" sx={{ 
                fontSize: '0.875rem',
                mt: 0.5,
                color: theme.palette.info.main // 保持蓝色
              }}>
                TT : (CN) 你好世界
              </Typography>
            </Box>
          )}

          {/* 7. 接收 - 图片消息（martin1 编码 SSTV）: 音频播放器 → 解码后的图片 → 图片分辨率 */}
          {!isSender && message.messageType === 'SSTV' && (
            <Box>
              {/* 音频播放器 */}
              <AudioPlayer duration="01:18" />
              
              {/* 解码后的图片 */}
              <Box sx={{ 
                textAlign: 'center',
                bgcolor: alpha(theme.palette.common.white, 0.1),
                borderRadius: '8px',
                p: 1,
                mt: 1,
                mb: 1
              }}>
                {'imageUrl' in message && message.imageUrl && (
                  <img 
                    src={message.imageUrl} 
                    alt="SSTV 图片" 
                    style={{ 
                      width: '100%',
                      height: 'auto', 
                      maxHeight: '180px',
                      objectFit: 'contain', 
                      borderRadius: '6px'
                    }} 
                  />
                )}
              </Box>
              
              {/* 图片分辨率 */}
              {'imageDimensions' in message && message.imageDimensions && (
                <Typography variant="body2" sx={{ 
                  fontSize: '0.875rem',
                  textAlign: 'center',
                  mt: 0.5,
                  color: bubbleTextColor
                }}>
                  分辨率 : {message.imageDimensions}
                </Typography>
              )}
            </Box>
          )}

          {/* 8. 接收 - 未识别消息: 仅显示音频播放器 */}
          {!isSender && message.messageType === 'Unknown' && (
            <Box>
              {/* 音频播放器 */}
              <AudioPlayer duration="01:18" />
            </Box>
          )}
        </Box>

        {/* Footer Row - 修改时间显示格式，给发送消息添加保存图标 */}
        <Box sx={{ 
          display: 'flex', 
          justifyContent: 'space-between', 
          alignItems: 'center', 
          px: 1.5, 
          pb: 1,
          mt: 0.5
        }}>
          <Typography variant="caption" sx={{ 
            fontSize: '0.7rem',
            opacity: 0.8
          }}>
            {message.timestamp}
            {/* 修改接收时间显示格式，只显示时间不显示日期 */}
            {message.receivedTimestamp && !isSender && ` (Rcvd: ${message.receivedTimestamp.split(' ')[1]})`}
          </Typography>
          
          <Box sx={{ display: 'flex', gap: 0.5 }}>
            {/* 发送消息的保存图标 */}
            {isSender && (
              <Tooltip title="保存">
                <IconButton 
                  size="small" 
                  sx={{ 
                    color: bubbleTextColor,
                    opacity: 0.8,
                    width: 24,
                    height: 24,
                    '&:hover': { opacity: 1 }
                  }}
                >
                  <SaveAltIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            )}
            
            {!isSender && (
              <>
                <Tooltip title="另存为">
                  <IconButton 
                    size="small" 
                    sx={{ 
                      color: bubbleTextColor,
                      opacity: 0.8,
                      width: 24,
                      height: 24,
                      '&:hover': { opacity: 1 }
                    }}
                  >
                    <SaveAltIcon fontSize="small" />
                  </IconButton>
                </Tooltip>
                <Tooltip title="回复">
                  <IconButton 
                    size="small" 
                    sx={{ 
                      color: bubbleTextColor,
                      opacity: 0.8,
                      width: 24,
                      height: 24,
                      '&:hover': { opacity: 1 }
                    }}
                  >
                    <ReplyIcon fontSize="small" />
                  </IconButton>
                </Tooltip>
              </>
            )}
          </Box>
        </Box>
      </Paper>
    </Box>
  );
};

// 定义任务显示状态的联合类型
type TaskDisplayStatus = 'idle' | 'running' | 'paused' | 'ended';

/**
 * 普通呼叫任务页面
 * 包含对话界面和联系人列表
 */
const GeneralCallTaskPage: React.FC = () => {
  const theme = useTheme(); 

  // 页面级状态
  const [aiSummary, setAiSummary] = React.useState<string>("这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。这是AI生成的呼叫内容摘要占位符。它应该能显示大约三行文本，如果内容超出，则会出现滚动条。"); // 增加文本长度以测试滚动
  const [isTranslationEnabledByUser, setIsTranslationEnabledByUser] = React.useState<boolean>(false);
  const [targetLanguage, setTargetLanguage] = React.useState<string>("English");
  const [currentPageDisplayLanguage, setCurrentPageDisplayLanguage] = React.useState<string>("中文"); 
  const [isSummaryExpanded, setIsSummaryExpanded] = React.useState<boolean>(false); // 默认折叠

  React.useEffect(() => {
    if (currentPageDisplayLanguage === "中文") {
      setTargetLanguage("English");
    } else if (currentPageDisplayLanguage === "英语") { 
      setTargetLanguage("中文");
    }
  }, [currentPageDisplayLanguage]);

  // 任务控制栏的状态模拟 (这些应该来自 props 或 store)
  const currentTaskDisplayStatusForControls = 'running' as TaskDisplayStatus; // 用于控制栏按钮的模拟状态
  const isTaskRunningForControls = currentTaskDisplayStatusForControls === 'running' || currentTaskDisplayStatusForControls === 'paused';
  const isTaskPausedForControls = currentTaskDisplayStatusForControls === 'paused';
  const isAiRespondingForControls = true; 

  const successColor = theme.palette.success.main;

  // 启动/结束按钮逻辑 (基于 currentTaskDisplayStatusForControls)
  const startEndButtonText = isTaskRunningForControls ? "结束" : "启动";
  const startEndButtonIcon = isTaskRunningForControls ? <StopRoundedIcon /> : <PlayArrowRoundedIcon />;
  const startEndButtonBgColor = isTaskRunningForControls 
    ? theme.palette.error.main 
    : '#a1efff'; 
  const startEndButtonHoverBgColor = isTaskRunningForControls 
    ? theme.palette.error.dark 
    : '#85e3f2';
  const startEndButtonTextColor = theme.palette.getContrastText(startEndButtonBgColor);

  // 暂停/恢复按钮逻辑 (基于 currentTaskDisplayStatusForControls)
  const pauseResumeButtonText = isTaskPausedForControls ? "恢复" : "暂停";
  const pauseResumeButtonIcon = isTaskPausedForControls ? <PlayArrowRoundedIcon /> : <PauseRoundedIcon />;
  const pauseResumeButtonDisabled = !isTaskRunningForControls;
  let pauseResumeButtonBgColor: string | undefined = undefined; 
  let pauseResumeButtonTextColor: string | undefined = undefined;
  let pauseResumeButtonHoverBgColor: string | undefined = undefined;

  if (isTaskRunningForControls) {
    if (isTaskPausedForControls) { 
      pauseResumeButtonBgColor = successColor; 
      pauseResumeButtonHoverBgColor = theme.palette.success.dark;
    } else { 
      pauseResumeButtonBgColor = theme.palette.warning.main;
      pauseResumeButtonHoverBgColor = theme.palette.warning.dark;
    }
    if (pauseResumeButtonBgColor) { 
        pauseResumeButtonTextColor = theme.palette.getContrastText(pauseResumeButtonBgColor);
    }
  }
  
  const aiResponseLabelColor = isAiRespondingForControls ? successColor : theme.palette.text.secondary;
  
  let runtimeTextColor = '';
  let runtimeBackgroundColor = '';
  if (currentTaskDisplayStatusForControls === 'running') {
    runtimeTextColor = successColor; 
    runtimeBackgroundColor = theme.palette.mode === 'dark' ? theme.palette.grey[800] : theme.palette.grey[100]; 
  } else if (currentTaskDisplayStatusForControls === 'paused') {
    runtimeTextColor = theme.palette.getContrastText(theme.palette.warning.main);
    runtimeBackgroundColor = theme.palette.warning.main;
  } else if (currentTaskDisplayStatusForControls === 'ended') {
    runtimeTextColor = theme.palette.error.contrastText; 
    runtimeBackgroundColor = theme.palette.error.main;
  } else { // idle
    runtimeTextColor = theme.palette.mode === 'dark' ? theme.palette.grey[300] : theme.palette.grey[700];
    runtimeBackgroundColor = theme.palette.mode === 'dark' ? alpha(theme.palette.common.white, 0.08) : alpha(theme.palette.common.black, 0.05);
  }

  return (
    <Box // 主页面容器
      sx={{
        flexGrow: 1, 
        height: '100%', 
        display: 'flex',
        flexDirection: 'row', 
        p: 0, 
      }}
    >
      {/* 左侧三级板块 (对话界面区) */}
      <Box
        sx={{
          flexGrow: 1, 
          height: '100%',
          display: 'flex',
          flexDirection: 'column', 
          borderRight: '1px solid', 
          borderColor: 'divider',
          pb: theme.spacing(0), // 添加这一行：为整个左侧面板添加底部padding，让底部区域下移
        }}
      >
        {/* 任务控制栏容器 */}
        <Box 
          sx={{
            display: 'flex',
            flexDirection: 'row',
            alignItems: 'center',
            justifyContent: 'space-between', 
            p: 1.5, 
            borderBottom: '1px solid',
            borderColor: 'divider',
            flexWrap: 'wrap', 
            gap: 1.5, 
          }}
        >
          {/* A. 左侧控制组: 启动/结束, 暂停/恢复, 运行时间 */}
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
            {/* i. 启动/结束按钮 */}
            <Button
              variant="contained"
              startIcon={startEndButtonIcon}
              sx={{
                borderRadius: '20px',
                py: 0.75, 
                lineHeight: 'normal', 
                px: 2, 
                textTransform: 'none',
                fontSize: '0.875rem',
                fontWeight: 500,
                backgroundColor: startEndButtonBgColor,
                color: startEndButtonTextColor,
                '&:hover': {
                  backgroundColor: startEndButtonHoverBgColor,
                },
              }}
            >
              {startEndButtonText}
            </Button>

            {/* ii. 暂停/恢复按钮 */}
            <Button
              variant="contained"
              startIcon={pauseResumeButtonIcon}
              disabled={pauseResumeButtonDisabled} 
              sx={{
                borderRadius: '20px',
                py: 0.75, 
                lineHeight: 'normal', 
                px: 2,
                textTransform: 'none',
                fontSize: '0.875rem',
                fontWeight: 500,
                backgroundColor: pauseResumeButtonDisabled ? undefined : pauseResumeButtonBgColor,
                color: pauseResumeButtonDisabled ? undefined : pauseResumeButtonTextColor,
                '&:hover': {
                  backgroundColor: pauseResumeButtonDisabled ? undefined : pauseResumeButtonHoverBgColor,
                },
              }}
            >
              {pauseResumeButtonText}
            </Button>

            {/* iii. 运行时间显示 (复古电子秒表风格 - 更新配色和对齐) */}
            <Box 
              sx={{ 
                ml: 1, 
                display: 'flex', 
                alignItems: 'center', 
                height: '34px', // 与按钮大致相同的高度
              }}
            >
              <Typography
                sx={{
                  fontFamily: '"Micro 5 Charted", monospace', 
                  fontWeight: 400, 
                  fontSize: '1.7rem', 
                  color: runtimeTextColor, // 使用动态文字颜色
                  backgroundColor: runtimeBackgroundColor, // 使用动态背景颜色
                  px: theme.spacing(1.5), 
                  py: theme.spacing(0.5), 
                  borderRadius: '8px',    
                  lineHeight: 1,        
                  letterSpacing: '0.05em',
                  textAlign: 'center',
                  display: 'flex',        
                  alignItems: 'center',   
                  justifyContent: 'center', 
                  height: '100%', 
                  boxSizing: 'border-box',
                  WebkitFontSmoothing: 'antialiased', // 尝试改善字体渲染
                  MozOsxFontSmoothing: 'grayscale',  // 尝试改善字体渲染
                }}
              >
                00:00:00
              </Typography>
            </Box>
          </Box>

          {/* B. 右侧控制组: AI应答开关 */}
          <Box sx={{ display: 'flex', alignItems: 'center' }}>
            <FormControlLabel
              control={<Switch color="primary" size="small" checked={isAiRespondingForControls} />} 
              label={
                <Typography sx={{ fontSize: '0.875rem', fontWeight: 500, color: aiResponseLabelColor, mr: 0.5 }}>
                  AI应答
                </Typography>
              }
              labelPlacement="start"
              sx={{ 
                mr: 0, 
              }}
            />
          </Box>
        </Box>

        {/* --- BEGIN FOURTH REFACTORED MESSAGE STREAM AREA --- */}
        <Box
          sx={{
            display: 'flex',
            flexDirection: 'column',
            flexGrow: 1,
            height: '100%', 
            overflow: 'hidden', 
          }}
        >
          {/* III. Refined AI Summary Card */}
          <Paper
            elevation={1}
            sx={{
              p: 1,
              m: 1.5,
              mb: 1,
              borderRadius: '10px',
              borderBottom: isSummaryExpanded ? `1px solid ${theme.palette.divider}` : 'none',
            }}
          >
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: isSummaryExpanded ? 1 : 0 }}> 
              <Typography variant="subtitle2" sx={{ color: theme.palette.text.secondary }}>
                当前任务摘要
              </Typography>
              <Box sx={{ display: 'flex', alignItems: 'center' }}>
                <Button 
                  variant="text" 
                  size="small"
                  startIcon={<AutoAwesomeOutlinedIcon fontSize="small" />}
                  sx={{
                    color: theme.palette.error.main,
                    textTransform: 'none',
                    fontWeight: 500,
                    px: 0.5,
                    '&:hover': {
                      backgroundColor: alpha(theme.palette.error.main, 0.08),
                    }
                  }}
                  onClick={() => {
                    setIsSummaryExpanded(true); // 点击生成摘要时展开
                    // TODO: 实现生成摘要逻辑，可能会用到 setAiSummary 
                  }}
                >
                  生成摘要
                </Button>
                <Tooltip title={isSummaryExpanded ? "收起摘要" : "展开摘要"}>
                  <IconButton
                    onClick={() => setIsSummaryExpanded(!isSummaryExpanded)}
                    size="small"
                    sx={{ 
                      ml: 0.5, 
                      color: theme.palette.text.secondary 
                    }}
                  >
                    {isSummaryExpanded ? <KeyboardArrowUpIcon /> : <KeyboardArrowDownIcon />}
                  </IconButton>
                </Tooltip>
              </Box>
            </Box>
            <Collapse in={isSummaryExpanded} timeout="auto" unmountOnExit>
              <Box
                sx={{
                  position: 'relative', // 创建相对定位容器
                  width: '100%', // 保持宽度100%
                  height: `calc(${theme.typography.body2.lineHeight} * 5em)`, // 固定高度
                }}
              >
                <Box
                  tabIndex={0}
                  sx={{
                    position: 'absolute', // 绝对定位
                    top: 0,
                    left: 0,
                    right: 0, // 撑满父容器宽度
                    bottom: 0, // 撑满父容器高度
                    backgroundColor: alpha(theme.palette.action.hover, 0.04),
                    padding: theme.spacing(1),
                    borderRadius: theme.shape.borderRadius,
                    overflowY: 'auto',
                    // 完全隐藏滚动条但保留功能
                    msOverflowStyle: 'none', // IE和Edge
                    scrollbarWidth: 'none', // Firefox
                    '&::-webkit-scrollbar': {
                      display: 'none', // Chrome, Safari, Opera
                    },
                    '&:focus': {
                      outline: `1px solid ${theme.palette.primary.main}`,
                    },
                  }}
                >
                  <Typography variant="body2" sx={{ whiteSpace: 'pre-wrap' }}>
                    {aiSummary}
                  </Typography>
                </Box>
              </Box>
            </Collapse>
          </Paper>

          {/* 在 AI 摘要卡片下方添加分割线 - 仅在展开时显示 */}
          {isSummaryExpanded && <Divider sx={{ mx: 1.5, mt: theme.spacing(1) }} />}

          {/* Sub-Control Bar (不再有 borderBottom 或 mt) */}
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              py: theme.spacing(0.5),
              px: theme.spacing(1.5),
            }}
          >
            <Typography variant="body2" sx={{ fontWeight: 500, color: theme.palette.text.secondary }}>
              任务启动时间: 2025-05-12 08:05:30
            </Typography>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: theme.spacing(1.5) }}>
              <FormControlLabel
                control={
                  <Switch
                    checked={isTranslationEnabledByUser}
                    onChange={() => setIsTranslationEnabledByUser(prev => !prev)}
                    size="small"
                    color={isTranslationEnabledByUser ? "success" : "default"}
                  />
                }
                label={
                  <Typography 
                    variant="body2" 
                    sx={{ 
                      fontWeight: 500,
                      color: isTranslationEnabledByUser ? theme.palette.success.main : theme.palette.text.secondary
                    }}
                  >
                    翻译
                  </Typography>
                }
                labelPlacement="start"
                sx={{ mr: 0 }} 
              />
              <FormControl size="small" variant="standard" sx={{ minWidth: 100 }}>
                <Select
                  value={targetLanguage}
                  onChange={(e) => setTargetLanguage(e.target.value as string)}
                  disableUnderline 
                  sx={{ 
                    fontSize: '0.875rem', 
                    '& .MuiSelect-select': { paddingRight: '24px' }, 
                    // 以下样式确保在 standard variant 下没有下划线
                    '&:before': { borderBottom: 'none' }, 
                    '&:hover:not(.Mui-disabled):before': { borderBottom: 'none' },
                    '&.Mui-focused:after': { borderBottom: 'none' },
                  }} 
                >
                  <MenuItem value="English">English</MenuItem>
                  <MenuItem value="日本語">日本語</MenuItem>
                  <MenuItem value="Español">Español</MenuItem>
                  <MenuItem value="Français">Français</MenuItem>
                  <MenuItem value="Deutsch">Deutsch</MenuItem>
                  <MenuItem value="中文">中文</MenuItem>
                </Select>
              </FormControl>
            </Box>
          </Box>

          {/* IV. Refined Core Message Display Area */}
          <Paper 
            elevation={1}
            sx={{
              flexGrow: 1,
              display: 'flex',
              flexDirection: 'column',
              overflow: 'hidden',
              borderRadius: '10px',
              mx: theme.spacing(1.5),
              mt: theme.spacing(0),
              mb: theme.spacing(0.5),
            }}
          >
            <Box // 内部滚动容器
              sx={{
                flexGrow: 1,
                overflowY: 'auto',
                p: 2,
                display: 'flex',
                flexDirection: 'column',
                gap: theme.spacing(1.5),
                backgroundColor: theme.palette.customSurfaces?.surface1 || theme.palette.background.paper,
                // IV.A. 消息区滚动条样式
                '&::-webkit-scrollbar': { width: '5px' },
                '&::-webkit-scrollbar-track': { background: 'transparent' },
                '&::-webkit-scrollbar-thumb': { 
                  backgroundColor: theme.palette.action.disabled, 
                  borderRadius: '10px' 
                },
                '&::-webkit-scrollbar-thumb:hover': { 
                  backgroundColor: theme.palette.action.active, 
                },
                scrollbarWidth: 'thin',
                scrollbarColor: `${theme.palette.action.disabled} transparent`,
              }}
            >
              {mockMessages.map((msg) => (
                <MessageBubble key={msg.id} message={msg} />
              ))}
            </Box>
          </Paper>

        </Box>
        {/* --- END FOURTH REFACTORED MESSAGE STREAM AREA --- */}

        {/* 底部固定区域 - 包含电平表和消息输入 */}
        <Box
          sx={{
            display: 'flex',
            flexDirection: 'column',
            mt: theme.spacing(-0.4), // 增加更大的上边距，让底部区域下移到红线位置 (约15-20px)
          }}
        >
          {/* 电平表区域 */}
          <Box // Container for Level Meters
            sx={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              px: theme.spacing(1.5),
              py: theme.spacing(0.5),
            }}
          >
            {/* RX Level Meter (Left) */}
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
              <Typography variant="caption" sx={{ fontWeight: 'medium', color: theme.palette.text.secondary }}>RX</Typography>
              {/* RX 电平表占位符 */}
              <Box sx={{ width: 100, height: 20, bgcolor: 'action.disabledBackground', borderRadius: 1, border: `1px solid ${theme.palette.divider}` }}>
                <Typography variant="caption" sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'text.disabled' }}>RX Meter</Typography>
              </Box>
            </Box>

            {/* TX Level Meter (Right) */}
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
              {/* TX 电平表占位符 */}
              <Box sx={{ width: 100, height: 20, bgcolor: 'action.disabledBackground', borderRadius: 1, border: `1px solid ${theme.palette.divider}` }}>
                <Typography variant="caption" sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'text.disabled' }}>TX Meter</Typography>
              </Box>
              {/* TX 标签移动到占位符之后 */}
              <Typography variant="caption" sx={{ fontWeight: 'medium', color: theme.palette.text.secondary, ml: 0.5 }}>TX</Typography>
            </Box>
          </Box>

          {/* 消息输入区域 */}
          <Box 
            sx={{ 
              borderTop: '1px solid', 
              borderColor: 'divider', 
              display: 'flex', 
              flexDirection: 'row',
              alignItems: 'stretch', 
              p: theme.spacing(1.5, 1.5, 0, 1.5), // 关键修改：将底部padding从1.5改为3，让整个输入区域下移
            }}
          >
            {/* 左侧：包含两行输入控件的 Stack */}
            <Stack direction="column" spacing={1} sx={{ flexGrow: 1, mr: 1.5 }}>
              {/* Row 1: Main Input Controls (Top Row) */}
              <Stack direction="row" alignItems="center" spacing={1} sx={{ width: '100%' }}>
                {/* Text Input Field with Internal IconButtons */}
                <TextField
                  variant="outlined"
                  size="small"
                  multiline
                  minRows={1}
                  maxRows={4}
                  placeholder="输入消息..."
                  sx={{ 
                    flexGrow: 1, 
                    bgcolor: theme.palette.customSurfaces?.surface1 || theme.palette.background.paper,
                    '& .MuiOutlinedInput-root': {
                      borderColor: theme.palette.success.main,
                      '&:hover fieldset': {
                        borderColor: theme.palette.success.light,
                      },
                      '&.Mui-focused fieldset': {
                        borderColor: theme.palette.success.dark,
                      },
                      paddingLeft: theme.spacing(1), // 给startAdornment留空间
                    },
                  }}
                  InputProps={{
                    startAdornment: (
                      <Stack direction="row" alignItems="center" spacing={1} sx={{ mr: 0.5 }}>
                        <Tooltip title="AI">
                          <IconButton size="small" sx={{ 
                              color: theme.palette.error.main  // 改为红色，与"生成摘要"的AI图标一致
                          }}>
                            <AutoAwesomeOutlinedIcon fontSize="small" />
                          </IconButton>
                        </Tooltip>
                        
                        <Tooltip title="插入图片">
                          <IconButton size="small" sx={{ 
                              color: theme.palette.warning.main, // 保持PIC的橙色
                          }}>
                            <ImageIcon fontSize="small" />
                          </IconButton>
                        </Tooltip>
                        
                        <Tooltip title="发送CW">
                          {/* 将CW图标改为圆角矩形，避免与CW key混淆 */}
                          <Box 
                            sx={{ 
                              bgcolor: theme.palette.secondary.main, // CW 紫色
                              color: theme.palette.secondary.contrastText,
                              fontSize: '0.7rem',
                              borderRadius: 1, // 圆角矩形
                              px: 0.75, // 水平内边距
                              py: 0.25, // 垂直内边距
                              display: 'flex',
                              alignItems: 'center',
                              justifyContent: 'center',
                              cursor: 'pointer'
                            }}
                          >
                            CW
                          </Box>
                        </Tooltip>
                        
                        <Chip 
                          label="Reply" 
                          size="small" 
                          disabled 
                          sx={{ 
                            bgcolor: theme.palette.action.disabledBackground,
                            color: theme.palette.text.disabled,
                            height: '22px'
                          }} 
                        />
                      </Stack>
                    ),
                  }}
                />
              </Stack>

              {/* Row 2: Preview Area with Integrated Controls */}
              <Paper
                variant="outlined"
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  p: theme.spacing(0.75, 1),
                  minHeight: '40px',
                  bgcolor: theme.palette.customSurfaces?.surface1 || alpha(theme.palette.action.hover, 0.5),
                  borderColor: theme.palette.divider,
                  borderRadius: '10px',
                  width: '100%',
                }}
              >
                {/* Left Side: Control Icons */}
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                  <Tooltip title="翻译">
                    <IconButton size="small" sx={{ color: theme.palette.text.secondary }}>
                      <TranslateIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                  <Tooltip title="TTS">
                    <IconButton size="small" sx={{ 
                        color: theme.palette.text.secondary,
                    }}>
                      <VolumeUpIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                  <Tooltip title="CW Key">
                    <Avatar 
                      sx={{ 
                        width: 28, 
                        height: 28, 
                        bgcolor: theme.palette.grey[600], // 默认为深灰色
                        color: theme.palette.getContrastText(theme.palette.grey[600]), 
                        fontSize: '0.75rem', 
                        cursor: 'pointer',
                        '&:hover': {
                          bgcolor: theme.palette.grey[500], // 悬停时变为稍亮的灰色
                        },
                        '&:active': {
                          bgcolor: theme.palette.error.main, // 点击按住时变为红色 (模拟电报键)
                          color: theme.palette.error.contrastText,
                        },
                        transition: 'background-color 0.1s', // 快速响应，模拟电报键的即时反馈
                        userSelect: 'none', // 防止文字被选中
                      }}
                    >
                      CW
                    </Avatar>
                  </Tooltip>
                </Box>

                {/* Center Area: Audio Player + Text Preview */}
                <Box sx={{ display: 'flex', alignItems: 'center', flexGrow: 1, mx: 1, gap: 1 }}>
                  {/* Audio Player Placeholder (红框位置) - 移除Paper包装和边框 */}
                  <Box
                    sx={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      minWidth: '120px',
                      height: '28px',
                      // 移除独立背景色和边框，与预览框背景一致
                    }}
                  >
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                      <IconButton size="small" sx={{ 
                          color: theme.palette.text.disabled,
                          padding: '2px',
                      }}>
                        <PlayArrowRoundedIcon fontSize="small" />
                      </IconButton>
                      <Typography variant="caption" sx={{ 
                          color: theme.palette.text.disabled,
                          fontSize: '0.7rem'
                      }}>
                        00:00
                      </Typography>
                      <Box sx={{ 
                          width: '60px', 
                          height: '2px', 
                          bgcolor: theme.palette.action.disabled,
                          borderRadius: '1px',
                          mx: 0.5
                      }} />
                    </Box>
                  </Box>

                  {/* 分隔符 | */}
                  <Typography variant="body2" sx={{ 
                      color: theme.palette.divider, 
                      fontSize: '1rem',
                      lineHeight: 1,
                      mx: 0.5
                  }}>
                    |
                  </Typography>

                  {/* Text Preview Area (绿框位置) - 可滚动文本 */}
                  <TextField
                    variant="standard"
                    size="small"
                    value="翻译后文字预览区域 - 这是一段比较长的文本用来测试横向滚动效果，这是一段比较长的文本用来测试横向滚动效果，这是一段比较长的文本用来测试横向滚动效果，这是一段比较长的文本用来测试横向滚动效果，这是一段比较长的文本用来测试横向滚动效果"
                    InputProps={{
                      readOnly: true,
                      disableUnderline: true,
                      sx: {
                        color: theme.palette.text.secondary,
                        fontStyle: 'italic',
                        fontSize: '0.875rem',
                        cursor: 'text',
                        '&:before': { borderBottom: 'none' },
                        '&:hover:not(.Mui-disabled):before': { borderBottom: 'none' },
                        '&.Mui-focused:after': { borderBottom: 'none' },
                        '& input': {
                          padding: 0,
                          height: '28px',
                          display: 'flex',
                          alignItems: 'center',
                          overflow: 'hidden',
                          textOverflow: 'clip', // 不显示省略号，直接截断
                          whiteSpace: 'nowrap',
                          '&:focus': {
                            outline: 'none',
                          }
                        }
                      }
                    }}
                    sx={{ 
                      flexGrow: 1, 
                      minHeight: '28px',
                      '& .MuiInputBase-root': {
                        height: '28px',
                      }
                    }}
                    onFocus={(e) => {
                      // 允许用户通过光标键来滚动查看文本
                      e.target.setSelectionRange(0, 0);
                    }}
                  />
                </Box>

                {/* Right Side: Additional Controls (if needed) */}
                <Box sx={{ display: 'flex', alignItems: 'center' }}>
                  {/* 这里可以放置其他控件，比如复制、清除等 */}
                </Box>
              </Paper>
            </Stack>

            {/* Send Button */}
            <Button
              variant="contained"
              sx={{ 
                borderRadius: '20px',
                px: 2.5, 
                minHeight: '74px', 
                bgcolor: '#a1efff',
                color: theme.palette.getContrastText('#a1efff'),
                '&:hover': { 
                  bgcolor: '#85e3f2',
                  transform: 'scale(1.02)',
                },
                transition: theme.transitions.create(['background-color', 'transform'], {
                  duration: theme.transitions.duration.short,
                }),
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                gap: theme.spacing(0.5),
              }}
            >
              <SendIcon sx={{ fontSize: '1.5rem' }} />
              <Typography variant="button" sx={{ fontSize: '1rem', fontWeight: 600 }}>
                发送
              </Typography>
            </Button>
          </Box>
        </Box>
      </Box>

      {/* 右侧三级板块 (联系人列表区) */}
      <Box
        sx={{
          width: '200px', 
          height: '100%',
          p: 1.5, 
          display: 'flex',
          flexDirection: 'column',
          backgroundColor: theme.palette.customSurfaces?.secondaryPaneBackground || theme.palette.background.default,
        }}
      >
        {/* 新的标题栏 */}
        <Box 
          sx={{ 
            display: 'flex', 
            alignItems: 'center', 
            justifyContent: 'space-between', 
            mb: 1.5 
          }}
        >
          <Tooltip title="添加好友">
            <IconButton size="small" sx={{ color: theme.palette.text.secondary }}>
              <AddCircleOutlineIcon />
            </IconButton>
          </Tooltip>
          <Typography 
            variant="subtitle1" 
            sx={{ 
              fontWeight: theme.typography.fontWeightMedium, 
              textAlign: 'center', 
              flexGrow: 1 
            }}
          >
            好友列表
          </Typography>
          <Typography 
            variant="body2" 
            sx={{ color: theme.palette.text.secondary, minWidth: '40px', textAlign: 'right' }} 
          >
            7/58
          </Typography>
        </Box>

        {/* 联系人搜索框 */}
        <TextField
          variant="outlined"
          size="small"
          placeholder="搜索联系人..."
          fullWidth
          InputProps={{
            startAdornment: (
              <InputAdornment position="start">
                <SearchIcon sx={{ color: theme.palette.action.active }} />
              </InputAdornment>
            ),
          }}
          sx={{ 
            mb: 1.5,
            '& .MuiOutlinedInput-root': {
              backgroundColor: theme.palette.customSurfaces?.secondaryPaneBackground || theme.palette.background.default,
              borderRadius: '20px', 
              '& fieldset': {
                borderRadius: '20px', 
              },
              '&:hover fieldset': {
                borderColor: theme.palette.primary.light,
              },
              '&.Mui-focused fieldset': {
                borderColor: theme.palette.primary.main,
              },
            },
          }}
        />

        {/* 联系人列表 */}
        <List
          disablePadding
          sx={{
            flexGrow: 1,
            overflowY: 'auto',
            width: '100%',
            '&::-webkit-scrollbar': { width: '6px' },
            '&::-webkit-scrollbar-track': { background: 'transparent' },
            '&::-webkit-scrollbar-thumb': { 
              backgroundColor: theme.palette.action.disabled, 
              borderRadius: '10px' 
            },
            '&::-webkit-scrollbar-thumb:hover': { 
              backgroundColor: theme.palette.action.active, 
            },
            scrollbarWidth: 'thin',
            scrollbarColor: `${theme.palette.action.disabled} transparent`,
          }}
        >
          {/* 联系人项 1 - VK7KSM */}
          <ListItemButton
            sx={{
              display: 'flex',
              alignItems: 'center',
              py: 0.75,
              px: 1,
              borderRadius: '8px',
              '&:hover': { 
                backgroundColor: theme.palette.action.hover 
              },
            }}
          >
            <Avatar 
              src={contactAvatar000} // VK7KSM
              sx={{ 
                width: 48, 
                height: 48, 
                mr: 2
              }} 
            />
            <Box sx={{ display: 'flex', flexDirection: 'column', flexGrow: 1, minWidth: 0 }}>
              <Typography 
                variant="body2" 
                noWrap
                sx={{ 
                  fontWeight: theme.typography.fontWeightMedium,
                  lineHeight: 1.3,
                }}
              >
                VK7KSM
              </Typography>
              <Typography 
                variant="caption" 
                noWrap
                sx={{ 
                  color: theme.palette.text.secondary,
                  lineHeight: 1.3, 
                  mt: theme.spacing(0.25)
                }}
              >
                在线
              </Typography>
            </Box>
            <Box sx={{ 
              width: 8, 
              height: 8, 
              borderRadius: '50%', 
              bgcolor: theme.palette.success.main,
              ml: 1 
            }} />
          </ListItemButton>
          <Divider component="li" sx={{ mx: 1, my: 0.5 }} />

          {/* 联系人项 2 - JA1ABC (使用图片头像) */}
          <ListItemButton
            sx={{
              display: 'flex',
              alignItems: 'center',
              py: 0.75,
              px: 1,
              borderRadius: '8px',
              '&:hover': { 
                backgroundColor: theme.palette.action.hover 
              },
            }}
          >
            <Avatar 
              src={contactAvatar001} // JA1ABC 使用 contactAvatar001
              sx={{ 
                width: 48, 
                height: 48, 
                mr: 2,
              }}
            />
            <Box sx={{ display: 'flex', flexDirection: 'column', flexGrow: 1, minWidth: 0 }}>
              <Typography 
                variant="body2" 
                noWrap
                sx={{ 
                  fontWeight: theme.typography.fontWeightMedium,
                  lineHeight: 1.3,
                }}
              >
                JA1ABC
              </Typography>
              <Typography 
                variant="caption" 
                noWrap
                sx={{ 
                  color: theme.palette.text.secondary,
                  lineHeight: 1.3,
                  mt: theme.spacing(0.25)
                }}
              >
                5分钟前
              </Typography>
            </Box>
          </ListItemButton>
          <Divider component="li" sx={{ mx: 1, my: 0.5 }} />

          {/* 联系人项 3 - DL2XYZ */}
          <ListItemButton
            sx={{
              display: 'flex',
              alignItems: 'center',
              py: 0.75,
              px: 1,
              borderRadius: '8px',
              // 最后一个元素下方不需要分隔符
              '&:hover': { 
                backgroundColor: theme.palette.action.hover 
              },
            }}
          >
            <Avatar 
              src={contactAvatar002} // DL2XYZ
              sx={{ 
                width: 48, 
                height: 48, 
                mr: 2 
              }} 
            />
            <Box sx={{ display: 'flex', flexDirection: 'column', flexGrow: 1, minWidth: 0 }}>
              <Typography 
                variant="body2" 
                noWrap
                sx={{ 
                  fontWeight: theme.typography.fontWeightMedium,
                  lineHeight: 1.3,
                }}
              >
                DL2XYZ
              </Typography>
              <Typography 
                variant="caption" 
                noWrap
                sx={{ 
                  color: theme.palette.text.secondary,
                  lineHeight: 1.3,
                  mt: theme.spacing(0.25)
                }}
              >
                离线
              </Typography>
            </Box>
          </ListItemButton>
          {/* 移除了 N0CALL 项及其 Divider */}
        </List>
      </Box>
    </Box>
  );
};

export default GeneralCallTaskPage; 