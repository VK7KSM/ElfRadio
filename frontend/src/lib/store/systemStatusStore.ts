import { create } from 'zustand';

// 定义 TypeScript 枚举/类型，以镜像后端 elfradio_types 的定义。
// 这些应与 crates/elfradio_types/src/lib.rs 中的 ConnectionStatus 和 SystemServiceStatus 对齐

export type ConnectionStatus = 'Connected' | 'Disconnected' | 'Checking' | 'Error' | 'Unknown';
export type SystemServiceStatus = 'Ok' | 'Warning' | 'Error' | 'Unknown';

export interface SystemStatusState {
  userUuid: string | null;
  radioStatus: ConnectionStatus;
  sdrStatus: ConnectionStatus;
  llmStatus: SystemServiceStatus;
  sttStatus: SystemServiceStatus;
  ttsStatus: SystemServiceStatus;
  translateStatus: SystemServiceStatus;
  networkStatus: ConnectionStatus;

  setUserUuid: (uuid: string | null) => void;
  setRadioStatus: (status: ConnectionStatus) => void;
  setSdrStatus: (status: ConnectionStatus) => void;
  setLlmStatus: (status: SystemServiceStatus) => void;
  setSttStatus: (status: SystemServiceStatus) => void;
  setTtsStatus: (status: SystemServiceStatus) => void;
  setTranslateStatus: (status: SystemServiceStatus) => void;
  setNetworkStatus: (status: ConnectionStatus) => void;
}

export const useSystemStatusStore = create<SystemStatusState>((set) => ({
  userUuid: null,
  radioStatus: 'Unknown',
  sdrStatus: 'Disconnected', // 根据占位符逻辑，SDR 默认为 Disconnected
  llmStatus: 'Unknown',
  sttStatus: 'Unknown',
  ttsStatus: 'Unknown',
  translateStatus: 'Unknown',
  networkStatus: 'Unknown', // 初始网络状态为 unknown，直到首次检查

  setUserUuid: (uuid) => set({ userUuid: uuid }),
  setRadioStatus: (status) => set({ radioStatus: status }),
  setSdrStatus: (status) => set({ sdrStatus: status }),
  setLlmStatus: (status) => set({ llmStatus: status }),
  setSttStatus: (status) => set({ sttStatus: status }),
  setTtsStatus: (status) => set({ ttsStatus: status }),
  setTranslateStatus: (status) => set({ translateStatus: status }),
  setNetworkStatus: (status) => set({ networkStatus: status }),
})); 