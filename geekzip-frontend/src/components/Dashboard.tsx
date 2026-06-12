import { useAppStore } from '../stores/appStore'
import DropZone from './DropZone'
import TaskList from './TaskList'
import Sidebar from './Sidebar'
import StatusBar from './StatusBar'
import SettingsPanel from './SettingsPanel'
import ExtractPage from '../pages/ExtractPage'
import CompressPage from '../pages/CompressPage'
import { EncryptPage, BatchPage, AnalyzePage, OrganizePage, CleanPage, DuplicatesPage } from '../pages/ToolPages'

const pages: Record<string, React.FC> = {
  inbox: DropZone,
  processing: TaskList,
  extracted: DropZone,
  favorites: DropZone,
  extract: ExtractPage,
  compress: CompressPage,
  encrypt: EncryptPage,
  batch: BatchPage,
  analyze: AnalyzePage,
  organize: OrganizePage,
  clean: CleanPage,
  duplicates: DuplicatesPage,
}

export default function App() {
  const showSettings = useAppStore((s) => s.showSettings)
  const hdrEnabled = useAppStore((s) => s.settings.hdrEnabled)
  const setShowSettings = useAppStore((s) => s.setShowSettings)
  const activeTab = useAppStore((s) => s.sidebarActive)

  const PageComponent = pages[activeTab] || DropZone

  return (
    <div className="h-screen w-screen flex flex-col bg-bg-primary text-text-primary overflow-hidden">
      <header className="h-12 flex items-center px-4 border-b border-border-subtle bg-bg-secondary shrink-0">
        <div className="flex items-center gap-3">
          <div className="flex gap-1.5">
            <div className="w-3 h-3 rounded-full bg-neon-red" />
            <div className="w-3 h-3 rounded-full bg-neon-orange" />
            <div className="w-3 h-3 rounded-full bg-neon-green" />
          </div>
          <span className="font-mono text-lg font-bold tracking-wider text-white ml-2">
            GeekZip
          </span>
          <span className="px-2 py-0.5 text-[10px] font-mono border border-neon-green text-neon-green rounded bg-neon-green/10"
            style={hdrEnabled ? { boxShadow: '0 0 8px rgba(0,230,118,0.3)' } : {}}>
            PRO
          </span>
        </div>

        <div className="flex-1 flex justify-center">
          <div className="flex bg-bg-tertiary rounded-lg p-0.5 border border-border-subtle">
            {['NORMAL', 'PRO', 'TERMINAL'].map((tab) => (
              <button
                key={tab}
                className={`px-4 py-1.5 text-xs font-mono rounded-md transition-all ${
                  tab === 'PRO'
                    ? 'bg-neon-green/20 text-neon-green border border-neon-green/50'
                    : 'text-text-muted hover:text-text-secondary'
                }`}
              >
                {tab}
              </button>
            ))}
          </div>
        </div>

        <div className="flex items-center gap-3">
          <button
            onClick={() => useAppStore.getState().setSettings({ hdrEnabled: !hdrEnabled })}
            className={`px-2 py-1 text-[10px] font-mono rounded border transition-all ${
              hdrEnabled
                ? 'border-neon-green text-neon-green bg-neon-green/10 shadow-[0_0_8px_rgba(0,230,118,0.3)]'
                : 'border-border-default text-text-muted'
            }`}
          >
            HDR {hdrEnabled ? 'ON' : 'OFF'}
          </button>

          <button
            onClick={() => setShowSettings(true)}
            className="text-text-muted hover:text-neon-green transition-colors"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
          <div className="w-8 h-8 rounded-full bg-bg-tertiary border border-border-default flex items-center justify-center">
            <span className="font-mono text-xs text-text-muted">G</span>
          </div>
        </div>
      </header>

      <div className="flex-1 flex overflow-hidden">
        <Sidebar />
        <main className="flex-1 flex flex-col overflow-auto">
          {(activeTab === 'inbox' || activeTab === 'processing' || activeTab === 'extracted' || activeTab === 'favorites') ? (
            <div className="flex-1 flex flex-col p-4 gap-4">
              <DropZone />
              <TaskList />
            </div>
          ) : (
            <PageComponent />
          )}
        </main>
      </div>

      <StatusBar />

      {showSettings && <SettingsPanel />}
    </div>
  )
}