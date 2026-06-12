import { useState } from 'react'
import { Package, Unlock, FolderOpen, ArrowRight } from 'lucide-react'
import { useAppStore } from '../stores/appStore'
import { useExtract } from '../hooks/useExtract'

export default function ExtractPage() {
  const settings = useAppStore((s) => s.settings)
  const { extract, loading } = useExtract()
  const [filePath, setFilePath] = useState('')
  const [targetDir, setTargetDir] = useState('')
  const [password, setPassword] = useState('')
  const [recursive, setRecursive] = useState(settings.recursiveEnabled)
  const [createSubfolder, setCreateSubfolder] = useState(settings.defaultCreateSubfolder)
  const [deleteAfter, setDeleteAfter] = useState(settings.defaultDeleteAfterExtract)
  const [result, setResult] = useState<any>(null)
  const [error, setError] = useState('')

  const handleExtract = async () => {
    if (!filePath) return
    setError('')
    setResult(null)
    try {
      const res = await extract(filePath, {
        targetDir: targetDir || undefined,
        password: password || undefined,
        recursive,
        deleteAfter,
      })
      setResult(res)
    } catch (err: any) {
      setError(String(err))
    }
  }

  return (
    <div className="flex-1 flex flex-col p-6 gap-6 overflow-auto">
      <h2 className="font-mono text-xl text-neon-green neon-text"
        style={{ textShadow: settings.hdrEnabled ? '0 0 10px rgba(0,230,118,0.8)' : 'none' }}>
        EXTRACT
      </h2>

      <div className="bg-bg-secondary border border-border-subtle rounded-lg p-6 space-y-5">
        <div className="space-y-2">
          <label className="font-mono text-xs text-text-muted uppercase tracking-wider">Archive File</label>
          <div className="flex gap-3">
            <input
              type="text"
              value={filePath}
              onChange={(e) => setFilePath(e.target.value)}
              placeholder="/path/to/archive.zip"
              className="flex-1 bg-bg-tertiary border border-border-default rounded-md font-mono text-sm text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-neon-green focus:shadow-[0_0_5px_rgba(0,230,118,0.3)] px-3 py-2 transition-all"
            />
            <button
              onClick={async () => {
                try {
                  const { invoke } = await import('@tauri-apps/api/core')
                  const path = await invoke('select_files', { multiple: false })
                  if (path) setFilePath(String(path))
                } catch {}
              }}
              className="px-4 py-2 bg-bg-tertiary border border-border-default rounded-md font-mono text-sm text-text-secondary hover:border-neon-green hover:text-neon-green transition-all flex items-center gap-2"
            >
              <FolderOpen className="w-4 h-4" />
              BROWSE
            </button>
          </div>
        </div>

        <div className="space-y-2">
          <label className="font-mono text-xs text-text-muted uppercase tracking-wider">Extract To</label>
          <input
            type="text"
            value={targetDir}
            onChange={(e) => setTargetDir(e.target.value)}
            placeholder="Leave empty for same directory"
            className="w-full bg-bg-tertiary border border-border-default rounded-md font-mono text-sm text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-neon-green focus:shadow-[0_0_5px_rgba(0,230,118,0.3)] px-3 py-2 transition-all"
          />
        </div>

        <div className="space-y-2">
          <label className="font-mono text-xs text-text-muted uppercase tracking-wider">Password (optional)</label>
          <div className="relative">
            <Unlock className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted" />
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Enter password if encrypted"
              className="w-full bg-bg-tertiary border border-border-default rounded-md font-mono text-sm text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-neon-green pl-10 pr-3 py-2 transition-all tracking-widest"
            />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <label className="flex items-center gap-2 cursor-pointer">
            <input type="checkbox" checked={createSubfolder} onChange={(e) => setCreateSubfolder(e.target.checked)}
              className="accent-[#00E676] w-4 h-4" />
            <span className="font-mono text-sm text-text-secondary">Create subfolder</span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input type="checkbox" checked={recursive} onChange={(e) => setRecursive(e.target.checked)}
              className="accent-[#00E676] w-4 h-4" />
            <span className="font-mono text-sm text-text-secondary">Recursive extract</span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input type="checkbox" checked={deleteAfter} onChange={(e) => setDeleteAfter(e.target.checked)}
              className="accent-[#00E676] w-4 h-4" />
            <span className="font-mono text-sm text-text-secondary">Delete after extract</span>
          </label>
        </div>

        <button
          onClick={handleExtract}
          disabled={loading || !filePath}
          className={`w-full py-3 rounded-md font-mono text-sm font-bold flex items-center justify-center gap-2 transition-all ${
            loading || !filePath
              ? 'bg-bg-active text-text-disabled border border-border-subtle cursor-not-allowed'
              : `bg-neon-green/10 border border-neon-green text-neon-green hover:bg-neon-green/20 hover:shadow-[0_0_20px_rgba(0,230,118,0.3)] ${settings.hdrEnabled ? 'hover:shadow-[0_0_30px_rgba(0,230,118,0.4)]' : ''}`
          }`}
        >
          <Package className="w-4 h-4" />
          {loading ? 'EXTRACTING...' : 'EXTRACT'}
          <ArrowRight className="w-4 h-4" />
        </button>
      </div>

      {result && (
        <div className="bg-neon-green/5 border border-neon-green/30 rounded-lg p-4">
          <p className="font-mono text-sm text-neon-green">Extraction successful!</p>
          <pre className="font-mono text-xs text-text-secondary mt-2 whitespace-pre-wrap">
            {JSON.stringify(result, null, 2)}
          </pre>
        </div>
      )}

      {error && (
        <div className="bg-neon-red/5 border border-neon-red/30 rounded-lg p-4">
          <p className="font-mono text-sm text-neon-red">Error: {error}</p>
        </div>
      )}
    </div>
  )
}