import {
  Inbox, Loader2, FolderOpen, Star,
  FolderInput, FolderOutput, Lock, Layers,
  Scan, FolderTree, Trash2, Copy,
  Settings
} from 'lucide-react'
import { useAppStore } from '../stores/appStore'

const archiveItems = [
  { id: 'inbox', label: 'Inbox', icon: Inbox, count: 0 },
  { id: 'processing', label: 'Processing', icon: Loader2, count: 0 },
  { id: 'extracted', label: 'Extracted', icon: FolderOpen, count: 0 },
  { id: 'favorites', label: 'Favorites', icon: Star, count: 0 },
]

const toolItems = [
  { id: 'extract', label: 'Extract', icon: FolderInput },
  { id: 'compress', label: 'Compress', icon: FolderOutput },
  { id: 'encrypt', label: 'Encrypt', icon: Lock },
  { id: 'batch', label: 'Batch', icon: Layers },
]

const aiItems = [
  { id: 'analyze', label: 'Analyze', icon: Scan },
  { id: 'organize', label: 'Organize', icon: FolderTree },
  { id: 'clean', label: 'Clean', icon: Trash2 },
  { id: 'duplicates', label: 'Duplicates', icon: Copy },
]

export default function Sidebar() {
  const active = useAppStore((s) => s.sidebarActive)
  const setActive = useAppStore((s) => s.setSidebarActive)
  const tasks = useAppStore((s) => s.tasks)

  const inboxCount = tasks.filter((t) => t.status === 'queued').length
  const processingCount = tasks.filter((t) => t.status === 'extracting').length
  const extractedCount = tasks.filter((t) => t.status === 'completed').length

  const counts: Record<string, number> = {
    inbox: inboxCount,
    processing: processingCount,
    extracted: extractedCount,
    favorites: 0,
  }

  return (
    <div className="w-56 bg-bg-secondary border-r border-border-subtle flex flex-col shrink-0">
      <div className="px-3 py-4">
        <h3 className="font-mono text-[10px] text-neon-green tracking-[0.2em] uppercase mb-3">
          Archives
        </h3>
        {archiveItems.map((item) => {
          const Icon = item.icon
          const isActive = active === item.id
          const count = counts[item.id] || item.count
          return (
            <button
              key={item.id}
              onClick={() => setActive(item.id)}
              className={`w-full flex items-center gap-3 px-3 py-2 rounded-md font-mono text-sm transition-all mb-0.5 ${
                isActive
                  ? 'bg-neon-green/10 text-neon-green border-l-2 border-neon-green'
                  : 'text-text-secondary hover:bg-bg-hover hover:text-text-primary border-l-2 border-transparent'
              }`}
            >
              <Icon className={`w-4 h-4 ${isActive ? 'text-neon-green' : 'text-text-muted'}`} />
              <span className="flex-1 text-left">{item.label}</span>
              {count > 0 && (
                <span className={`text-xs ${isActive ? 'text-neon-green' : 'text-text-muted'}`}>
                  {count}
                </span>
              )}
            </button>
          )
        })}
      </div>

      <div className="px-3 py-4 border-t border-border-subtle">
        <h3 className="font-mono text-[10px] text-neon-green tracking-[0.2em] uppercase mb-3">
          Tools
        </h3>
        {toolItems.map((item) => {
          const Icon = item.icon
          const isActive = active === item.id
          return (
            <button
              key={item.id}
              onClick={() => setActive(item.id)}
              className={`w-full flex items-center gap-3 px-3 py-2 rounded-md font-mono text-sm transition-all mb-0.5 ${
                isActive
                  ? 'bg-neon-green/10 text-neon-green border-l-2 border-neon-green'
                  : 'text-text-secondary hover:bg-bg-hover hover:text-text-primary border-l-2 border-transparent'
              }`}
            >
              <Icon className={`w-4 h-4 ${isActive ? 'text-neon-green' : 'text-text-muted'}`} />
              <span>{item.label}</span>
            </button>
          )
        })}
      </div>

      <div className="px-3 py-4 border-t border-border-subtle">
        <h3 className="font-mono text-[10px] text-neon-green tracking-[0.2em] uppercase mb-3">
          AI Assistant
        </h3>
        {aiItems.map((item) => {
          const Icon = item.icon
          const isActive = active === item.id
          return (
            <button
              key={item.id}
              onClick={() => setActive(item.id)}
              className={`w-full flex items-center gap-3 px-3 py-2 rounded-md font-mono text-sm transition-all mb-0.5 ${
                isActive
                  ? 'bg-neon-green/10 text-neon-green border-l-2 border-neon-green'
                  : 'text-text-secondary hover:bg-bg-hover hover:text-text-primary border-l-2 border-transparent'
              }`}
            >
              <Icon className={`w-4 h-4 ${isActive ? 'text-neon-green' : 'text-text-muted'}`} />
              <span>{item.label}</span>
            </button>
          )
        })}
      </div>

      <div className="mt-auto p-4 border-t border-border-subtle">
        <button className="w-full bg-bg-tertiary hover:bg-bg-hover rounded-lg p-3 text-center transition-all">
          <Settings className="w-5 h-5 text-text-muted mx-auto mb-1" />
          <span className="font-mono text-xs text-text-muted">Settings</span>
        </button>
      </div>
    </div>
  )
}