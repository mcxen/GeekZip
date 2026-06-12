import { useCallback } from 'react'
import { Package, FileInput, FolderInput } from 'lucide-react'
import { useAppStore } from '../stores/appStore'
import { useExtract } from '../hooks/useExtract'

export default function DropZone() {
  const isDragging = useAppStore((s) => s.isDragging)
  const setDragging = useAppStore((s) => s.setIsDragging)
  const hdrEnabled = useAppStore((s) => s.settings.hdrEnabled)
  const { extract } = useExtract()

  const handleDrop = useCallback(async (e: React.DragEvent) => {
    e.preventDefault()
    setDragging(false)
    const files = Array.from(e.dataTransfer.files)
    for (const file of files) {
      try {
        const path = (file as any).path || file.name
        await extract(path)
      } catch (err) {
        console.error('Extract failed:', err)
      }
    }
  }, [extract, setDragging])

  const handleFileInput = useCallback(async () => {
    const input = document.createElement('input')
    input.type = 'file'
    input.multiple = true
    input.accept = '.zip,.rar,.7z,.tar,.gz,.bz2,.xz,.zst,.lz4,.tgz,.tbz2,.txz'
    input.onchange = async () => {
      if (input.files) {
        for (const file of Array.from(input.files)) {
          try {
            const path = (file as any).path || file.name
            await extract(path)
          } catch (err) {
            console.error(err)
          }
        }
      }
    }
    input.click()
  }, [extract])

  return (
    <div
      className={`flex-1 flex flex-col items-center justify-center rounded-lg border-2 border-dashed transition-all duration-300 cursor-pointer relative overflow-hidden ${
        isDragging
          ? 'border-neon-green bg-neon-green/5'
          : 'border-border-default bg-bg-secondary/50 hover:border-neon-green/50'
      } ${hdrEnabled && isDragging ? 'shadow-[0_0_60px_rgba(0,230,118,0.3)]' : ''}`}
      onDragEnter={(e) => { e.preventDefault(); setDragging(true) }}
      onDragLeave={() => setDragging(false)}
      onDragOver={(e) => e.preventDefault()}
      onDrop={handleDrop}
      onClick={handleFileInput}
    >
      <div className="absolute inset-0 pointer-events-none"
        style={{
          backgroundImage: 'radial-gradient(circle, rgba(0,230,118,0.12) 1px, transparent 1px)',
          backgroundSize: '20px 20px',
        }}
      />

      {hdrEnabled && (
        <div className="absolute inset-0 pointer-events-none"
          style={{
            background: 'radial-gradient(ellipse at center, rgba(0,230,118,0.08) 0%, transparent 70%)',
          }}
        />
      )}

      <div className="relative z-10 flex flex-col items-center">
        <div className={`mb-6 relative ${hdrEnabled ? 'animate-[glowPulse_3s_ease-in-out_infinite]' : ''}`}
          style={hdrEnabled ? { boxShadow: '0 0 30px rgba(0,230,118,0.5), 0 0 60px rgba(0,230,118,0.2)' } : {}}>
          <div className="w-20 h-24 border-2 border-neon-green rounded-lg flex flex-col items-center justify-center relative bg-bg-tertiary/50">
            <div className="absolute top-0 left-0 right-0 h-6 border-b border-neon-green/50 bg-neon-green/10 rounded-t-lg" />
            <Package className="w-8 h-8 text-neon-green mt-4" />
          </div>
          <div className="absolute -bottom-2 left-1/2 -translate-x-1/2 w-16 h-1 bg-neon-green/30 rounded-full blur-sm" />
        </div>

        <h2 className="text-3xl text-neon-green mb-4 text-center"
          style={{
            fontFamily: "'VT323', monospace",
            letterSpacing: '0.05em',
            ...(hdrEnabled ? { textShadow: '0 0 20px rgba(0,230,118,0.8), 0 0 40px rgba(0,230,118,0.4), 0 0 80px rgba(0,230,118,0.2)' } : {})
          }}>
          DROP ARCHIVES HERE
        </h2>

        <p className="font-mono text-sm text-text-muted mb-8 tracking-widest uppercase">
          OR CLICK TO BROWSE FILES
        </p>

        <div className="flex gap-4">
          <button
            className={`px-6 py-3 bg-bg-tertiary border border-border-default rounded-md font-mono text-sm text-text-secondary hover:border-neon-green hover:text-neon-green transition-all duration-300 flex items-center gap-2 ${
              hdrEnabled ? 'hover:shadow-[0_0_20px_rgba(0,230,118,0.3)]' : ''
            }`}
            onClick={(e) => { e.stopPropagation(); handleFileInput() }}
          >
            <FileInput className="w-4 h-4" />
            IMPORT FILES
          </button>
          <button
            className={`px-6 py-3 bg-bg-tertiary border border-border-default rounded-md font-mono text-sm text-text-secondary hover:border-neon-green hover:text-neon-green transition-all duration-300 flex items-center gap-2 ${
              hdrEnabled ? 'hover:shadow-[0_0_20px_rgba(0,230,118,0.3)]' : ''
            }`}
            onClick={(e) => e.stopPropagation()}
          >
            <FolderInput className="w-4 h-4" />
            IMPORT FOLDER
          </button>
        </div>

        <div className="mt-8 flex gap-3">
          {['ZIP', 'RAR', '7Z', 'TAR', 'GZ', 'BZ2', 'XZ'].map((fmt) => (
            <span key={fmt} className="px-2 py-0.5 text-xs font-mono bg-bg-tertiary border border-border-subtle rounded text-text-muted">
              {fmt}
            </span>
          ))}
        </div>
      </div>
    </div>
  )
}