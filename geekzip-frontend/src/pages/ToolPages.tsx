import { useAppStore } from '../stores/appStore'
import { Lock, Scan, FolderTree, Trash2, Copy } from 'lucide-react'

export function EncryptPage() {
  return (
    <div className="flex-1 flex flex-col p-6 gap-6 overflow-auto">
      <h2 className="font-mono text-xl text-neon-green">ENCRYPT</h2>
      <div className="bg-bg-secondary border border-border-subtle rounded-lg p-12 flex flex-col items-center justify-center gap-4">
        <Lock className="w-12 h-12 text-text-muted" />
        <p className="font-mono text-text-muted text-center">Archive encryption is coming soon.</p>
        <p className="font-mono text-xs text-text-disabled text-center">Password-protect your archives with AES-256 encryption.</p>
      </div>
    </div>
  )
}

export function BatchPage() {
  return (
    <div className="flex-1 flex flex-col p-6 gap-6 overflow-auto">
      <h2 className="font-mono text-xl text-neon-green">BATCH</h2>
      <div className="bg-bg-secondary border border-border-subtle rounded-lg p-12 flex flex-col items-center justify-center gap-4">
        <Copy className="w-12 h-12 text-text-muted" />
        <p className="font-mono text-text-muted text-center">Batch processing is coming soon.</p>
        <p className="font-mono text-xs text-text-disabled text-center">Select multiple archives and process them all at once.</p>
      </div>
    </div>
  )
}

export function AnalyzePage() {
  return (
    <div className="flex-1 flex flex-col p-6 gap-6 overflow-auto">
      <h2 className="font-mono text-xl text-neon-green">ANALYZE</h2>
      <div className="bg-bg-secondary border border-border-subtle rounded-lg p-12 flex flex-col items-center justify-center gap-4">
        <Scan className="w-12 h-12 text-text-muted" />
        <p className="font-mono text-text-muted text-center">AI analysis is coming soon.</p>
        <p className="font-mono text-xs text-text-disabled text-center">Analyze archive contents, detect duplicates, and get cleanup suggestions.</p>
      </div>
    </div>
  )
}

export function OrganizePage() {
  return (
    <div className="flex-1 flex flex-col p-6 gap-6 overflow-auto">
      <h2 className="font-mono text-xl text-neon-green">ORGANIZE</h2>
      <div className="bg-bg-secondary border border-border-subtle rounded-lg p-12 flex flex-col items-center justify-center gap-4">
        <FolderTree className="w-12 h-12 text-text-muted" />
        <p className="font-mono text-text-muted text-center">Smart organizing is coming soon.</p>
      </div>
    </div>
  )
}

export function CleanPage() {
  return (
    <div className="flex-1 flex flex-col p-6 gap-6 overflow-auto">
      <h2 className="font-mono text-xl text-neon-green">CLEAN</h2>
      <div className="bg-bg-secondary border border-border-subtle rounded-lg p-12 flex flex-col items-center justify-center gap-4">
        <Trash2 className="w-12 h-12 text-text-muted" />
        <p className="font-mono text-text-muted text-center">Cleanup is coming soon.</p>
      </div>
    </div>
  )
}

export function DuplicatesPage() {
  return (
    <div className="flex-1 flex flex-col p-6 gap-6 overflow-auto">
      <h2 className="font-mono text-xl text-neon-green">DUPLICATES</h2>
      <div className="bg-bg-secondary border border-border-subtle rounded-lg p-12 flex flex-col items-center justify-center gap-4">
        <Copy className="w-12 h-12 text-text-muted" />
        <p className="font-mono text-text-muted text-center">Duplicate detection is coming soon.</p>
      </div>
    </div>
  )
}