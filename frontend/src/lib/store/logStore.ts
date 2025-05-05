import { create } from 'zustand';

// Define the structure of a single log message (can be simple string for now)
// Or use a more structured object: interface LogMessage { timestamp: Date; level: string; message: string; }
type LogMessage = string; // Keep it simple for now

interface LogState {
  messages: LogMessage[];
  maxMessages: number; // Limit the number of messages stored
  addMessage: (message: LogMessage) => void;
  clearLogs: () => void;
}

export const useLogStore = create<LogState>((set) => ({
  messages: [],
  maxMessages: 200, // Store latest 200 messages, adjust as needed
  addMessage: (message) => set((state) => {
    const newMessages = [...state.messages, message];
    // Trim messages if exceeding max length
    if (newMessages.length > state.maxMessages) {
      const excess = newMessages.length - state.maxMessages;
      newMessages.splice(0, excess); // Remove the oldest message(s)
    }
    return { messages: newMessages };
  }),
  clearLogs: () => set({ messages: [] }),
}));

// 3. Remove or Comment Out Incorrect Hook:
// export const useCurrentLogs = () => useLogStore((state) => state.logs); // Note: State field is 'messages', not 'logs' - correcting hook

// 4. Verify Correct Hook:
export const useLogMessages = () => useLogStore((state) => state.messages);

console.log('logStore.ts loaded'); 