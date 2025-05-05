import { create } from 'zustand';

// Define basic types for Task (matches backend types)
export type TaskStatus = 'Idle' | 'Running' | 'Stopping';

// Task 模式类型（与后端对应）
export type TaskMode = 
  | 'GeneralCommunication'
  | 'AirbandListening'
  | 'SatelliteCommunication'
  | 'EmergencyCommunication'
  | 'MeshtasticGateway'
  | 'SimulatedQsoPractice';

export interface TaskInfo {
  id: string;
  name: string;
  mode: TaskMode;
  // 其他可能的字段 (参考后端 TaskInfo 结构体)
  // start_time?: Date;
  // is_simulation?: boolean;
}

interface TaskState {
  status: TaskStatus;
  selectedMode: TaskMode | null; // 添加选定的任务模式
  activeTaskInfo: TaskInfo | null;
  setSelectedMode: (mode: TaskMode | null) => void; // 添加设置选定模式的函数
  setTaskRunning: (taskInfo: TaskInfo) => void;
  setTaskStopping: () => void;
  setTaskIdle: () => void;
  resetSelectedMode: () => void; // 添加重置选定模式的函数
}

export const useTaskStore = create<TaskState>((set) => ({
  status: 'Idle', // 初始状态
  selectedMode: null, // 初始无选定模式
  activeTaskInfo: null,
  setSelectedMode: (mode) => set({ selectedMode: mode }),
  setTaskRunning: (taskInfo) => set({ status: 'Running', activeTaskInfo: taskInfo }),
  setTaskStopping: () => set((state) => {
    // 停止过程中保留任务信息，空闲时清除它
    if (state.status === 'Running') {
      return { status: 'Stopping' };
    }
    return {}; // 如果已经是 Idle 或 Stopping 状态则不变
  }),
  setTaskIdle: () => set({ status: 'Idle', activeTaskInfo: null }), // 空闲时清除任务信息
  resetSelectedMode: () => set({ selectedMode: null }), // 重置选定模式
}));

// 选择器钩子函数
export const useCurrentTaskStatus = () => useTaskStore((state) => state.status);
export const useActiveTaskInfo = () => useTaskStore((state) => state.activeTaskInfo);
export const useSelectedMode = () => useTaskStore((state) => state.selectedMode);

console.log('taskStore.ts loaded'); 