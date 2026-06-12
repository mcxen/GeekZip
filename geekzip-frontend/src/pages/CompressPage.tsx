import { useState } from 'react'
import { Package, ArrowRight } from 'lucide-react'
import { useAppStore } from '../stores/appStore'

export default function CompressPage() {
  const settings = useAppStore((s) => s.settings)
  const [paths, setPaths] = useState<string[]>([])
  const [output, setOutput] = useState('')
  const [format, setFormat] = useState('zip')
  const [loading, setLoading] = useState(false)
  const [result, setResult] = useState('')
  const [error, setError] = useState('')

  const handleCompress = async () => {
    if (paths.length === 0 || !output) return
    setLoading(true)
    setError('')
    setResult('')
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const res = await invoke('compress_files', { paths, output, format })
      setResult(String(res))
    } catch (err: any) {
      setError(String(err))
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex-1 flex flex-col p-6 gap-6 overflow-auto">
      <h2 className="font-mono text-xl text-neon-green"
        style={{ textShadow: settings.hdrEnabled ? '0 0 10px rgba(0,230,118,0.8)' : 'none' }}>
        COMPRESS
      </h2>

      <div className="bg-bg-secondary border border-border-subtle rounded-lg p-6 space-y-5">
        <div className="space-y-2">
          <label className="font-mono text-xs text-text-muted uppercase tracking-wider">Source Files</label>
          <input
            type="text"
            value={paths.join(', ')}
            onChange={(e) => setPaths(e.target.value.split(',').map(s => s.trim()).filter(Boolean))}
            placeholder="/path/to/file1, /path/to/file2"
            className="w-full bg-bg-tertiary border border-border-default rounded-md font-mono text-sm text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-neon-green px-3 py-2 transition-all"
          />
        </div>

        <div className="space-y-2">
          <label className="font-mono text-xs text-text-muted uppercase tracking-wider">Output Path</label>
          <input
            type="text"
            value={output}
            onChange={(e) => setOutput(e.target.value)}
            placeholder="/path/to/output.zip"
            className="w-full bg-bg-tertiary border border-border-default rounded-md font-mono text-sm text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-neon-green px-3 py-2 transition-all"
          />
        </div>

        <div className="space-y-2">
          <label className="font-mono text-xs text-text-muted uppercase tracking-wider">Format</label>
          <div className="flex gap-2">
            {['zip', 'tar.gz', 'tar.bz2', 'tar.xz', 'tar'].map((f) => (
              <button
                key={f}
                onClick={() => setFormat(f)}
                className={`px-3 py-1.5 text-xs font-mono rounded-md border transition-all ${
                  format === f
                    ? 'border-neon-green text-neon-green bg-neon-green/10'
                    : 'border-border-default text-text-muted hover:border-neon-green/50'
                }`}
              >
                {f.toUpperCase()}
              </button>
            ))}
          </div>
        </div>

        <button
          onClick={handleCompress}
          disabled={loading || paths.length === 0 || !output}
          className={`w-full py-3 rounded-md font-mono text-sm font-bold flex items-center justify-center gap-2 transition-all ${
            loading || paths.length === 0 || !output
              ? 'bg-bg-active text-text-disabled border border-border-subtle cursor-not-allowed'
              : 'bg-neon-green/10 border border-neon-green text-neon-green hover:bg-neon-green/20'
          }`}
        >
          <Package className="w-4 h-4" />
          {loading ? 'COMPRESSING...' : 'COMPRESS'}
          <ArrowRight className="w-4 h-4" />
        </button>
      </div>

      {result && (
        <div className="bg-neon-green/5 border border-neon-green/30 rounded-lg p-4">
          <p className="font-mono text-sm text-neon-green">Compression successful: {result}</p>
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