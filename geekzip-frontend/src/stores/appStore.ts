import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export interface Task {
  id: string
  source: string
  name: string
  size: number
  format: string
  status: 'queued' | 'analyzing' | 'extracting' | 'completed' | 'error'
  progress: number
  speed: string
  eta: string
  files: string[]
  targetDir: string
  passwordUsed?: string
  errorMessage?: string
  createdAt: number
}

export interface AppSettings {
  language: string
  hdrEnabled: boolean
  theme: string
  defaultExtractPath: string
  defaultOverwrite: string
  defaultCreateSubfolder: boolean
  defaultOpenAfterExtract: boolean
  defaultDeleteAfterExtract: boolean
  defaultDeleteIntermediate: boolean
  recursiveEnabled: boolean
  recursiveMaxDepth: number
  singleFileSizeLimit: number
  totalExtractSizeLimit: number
  passwordTimeoutSeconds: number
  autoSavePasswords: boolean
  useBuiltinPasswords: boolean
  usePasswordDictionary: boolean
  passwordDictionaryPath: string
  showCompletionNotification: boolean
  showErrorNotification: boolean
  enableWatcher: boolean
  watchPaths: string[]
  maxConcurrentTasks: number
  maxThreads: number
}

const defaultSettings: AppSettings = {
  language: 'zh',
  hdrEnabled: true,
  theme: 'dark',
  defaultExtractPath: 'current',
  defaultOverwrite: 'rename',
  defaultCreateSubfolder: true,
  defaultOpenAfterExtract: false,
  defaultDeleteAfterExtract: false,
  defaultDeleteIntermediate: false,
  recursiveEnabled: true,
  recursiveMaxDepth: 10,
  singleFileSizeLimit: 10,
  totalExtractSizeLimit: 50,
  passwordTimeoutSeconds: 30,
  autoSavePasswords: true,
  useBuiltinPasswords: true,
  usePasswordDictionary: false,
  passwordDictionaryPath: '',
  showCompletionNotification: true,
  showErrorNotification: true,
  enableWatcher: false,
  watchPaths: [],
  maxConcurrentTasks: 4,
  maxThreads: 8,
}

interface AppState {
  tasks: Task[]
  sidebarActive: string
  isDragging: boolean
  settings: AppSettings
  showSettings: boolean

  addTask: (task: Task) => void
  updateTask: (id: string, update: Partial<Task>) => void
  removeTask: (id: string) => void
  setSidebarActive: (item: string) => void
  setIsDragging: (v: boolean) => void
  setSettings: (s: Partial<AppSettings>) => void
  setShowSettings: (v: boolean) => void
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      tasks: [],
      sidebarActive: 'inbox',
      isDragging: false,
      settings: defaultSettings,
      showSettings: false,

      addTask: (task) =>
        set((state) => ({ tasks: [...state.tasks, task] })),

      updateTask: (id, update) =>
        set((state) => ({
          tasks: state.tasks.map((t) => (t.id === id ? { ...t, ...update } : t)),
        })),

      removeTask: (id) =>
        set((state) => ({ tasks: state.tasks.filter((t) => t.id !== id) })),

      setSidebarActive: (item) => set({ sidebarActive: item }),
      setIsDragging: (v) => set({ isDragging: v }),
      setSettings: (s) =>
        set((state) => ({ settings: { ...state.settings, ...s } })),
      setShowSettings: (v) => set({ showSettings: v }),
    }),
    {
      name: 'geekzip-settings',
      partialize: (state) => ({ settings: state.settings }),
    }
  )
)