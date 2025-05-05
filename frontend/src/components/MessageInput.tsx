import React, { useState } from 'react';
import {
  TextField,
  Box,
  IconButton,
} from '@mui/material';
import SendIcon from '@mui/icons-material/Send';

interface MessageInputProps {
  /**
   * Callback function triggered when the user sends a message.
   * @param text The message text entered by the user.
   */
  onSend: (text: string) => void;
  /**
   * Optional flag to disable input and send button while processing.
   */
  isLoading?: boolean;
}

const MessageInput: React.FC<MessageInputProps> = ({ onSend, isLoading = false }) => {
  const [inputText, setInputText] = useState("");

  const handleSendClick = () => {
    const trimmedText = inputText.trim();
    if (trimmedText && !isLoading) {
      onSend(trimmedText);
      setInputText(""); // Clear input after sending
    }
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    // Send on Enter press without Shift, unless loading
    if (event.key === 'Enter' && !event.shiftKey && !isLoading) {
      event.preventDefault(); // Prevent default Enter behavior (new line)
      handleSendClick();
    }
  };

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        p: 2, // Padding around the input area
        borderTop: 1, // Add a top border
        borderColor: 'divider', // Use theme divider color
        bgcolor: 'background.paper', // Match background if needed
      }}
    >
      <TextField
        multiline
        variant="outlined" // Or use "filled" for a slightly different look
        size="small" // Make the text field compact
        placeholder="输入消息... (Shift+Enter 换行)" // Updated placeholder
        value={inputText}
        onChange={(e) => setInputText(e.target.value)}
        onKeyDown={handleKeyDown} // Handle Enter key press
        fullWidth // Take up available width
        disabled={isLoading} // Disable when loading/sending
        maxRows={4} // Limit the maximum number of rows before scrolling
        sx={{
          flexGrow: 1, // Allow TextField to grow
          mr: 1, // Margin to the right, before the button
        }}
      />
      <IconButton
        color="primary" // Use primary theme color for the button
        disabled={!inputText.trim() || isLoading} // Disable if input is empty or loading
        onClick={handleSendClick}
        aria-label="Send message" // Accessibility label
      >
        <SendIcon />
      </IconButton>
    </Box>
  );
};

export default MessageInput;
