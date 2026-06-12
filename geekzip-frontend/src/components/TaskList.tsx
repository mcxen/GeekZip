import { Pause, X, Lock, CheckCircle, AlertCircle } from 'lucide-react'
import { useAppStore } from '../stores/appStore'

const statusColors: Record<string, string> = {
  analyzing: 'text-neon-blue',
  extracting: 'text-neon-orange',
  completed: 'text-neon-green',
  error: 'text-neon-red',
  queued: 'text-text-muted',
}

const statusLabels: Record<string, string> = {
  analyzing: 'Analyzing...',
  extracting: 'Extracting...',
  completed: 'Completed',
  error: 'Error',
  queued: 'Queued',
}

function DotProgress({ progress, hdrEnabled }: { progress: number; hdrEnabled: boolean }) {
  const dots = 20
  const filled = Math.round((progress / 100) * dots)
  return (
    <div className="flex items-center gap-3 w-full">
      <div className="flex-1 flex gap-1">
        {Array.from({ length: dots }, (_, i) => (
          <div
            key={i}
            className={`h-2 flex-1 rounded-sm transition-all duration-200 ${
              i < filled
                ? 'bg-neon-green'
                : 'bg-bg-active'
            }`}
            style={i < filled && hdrEnabled ? {
              boxShadow: '0 0 6px rgba(0,230,118,0.6)',
            } : {}}
          />
        ))}
      </div>
      <span className="font-mono text-sm text-neon-green w-12 text-right shrink-0">
        {progress}%
      </span>
    </div>
  )
}

export default function TaskList() {
  const tasks = useAppStore((s) => s.tasks)
  const removeTask = useAppStore((s) => s.removeTask)
  const hdrEnabled = useAppStore((s) => s.settings.hdrEnabled)

  if (tasks.length === 0) return null

  return (
    <div className="space-y-3">
      <h3 className="font-mono text-xs text-text-muted uppercase tracking-wider">
        Processing ({tasks.length})
      </h3>
      {tasks.map((task) => (
        <div
          key={task.id}
          className={`p-4 bg-bg-secondary border rounded-lg transition-all group ${
            task.status === 'completed'
              ? 'border-neon-green/30'
              : task.status === 'error'
                ? 'border-neon-red/30'
                : 'border-border-subtle hover:border-border-default'
          } ${hdrEnabled && task.status === 'completed' ? 'shadow-[0_0_15px_rgba(0,230,118,0.15)]' : ''}`}
        >
          <div className="flex items-start gap-4">
            <div className={`w-12 h-12 rounded-lg flex items-center justify-center border shrink-0 ${
              task.status === 'completed'
                ? 'border-neon-green bg-neon-green/10'
                : task.status === 'error'
                  ? 'border-neon-red bg-neon-red/10'
                  : 'border-neon-blue bg-neon-blue/10'
            }`}>
              <span className="font-mono text-xs font-bold text-text-secondary">
                {task.format || '?'}
              </span>
            </div>

            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between mb-1">
                <h4 className="font-mono text-sm text-neon-green truncate">
                  {task.name}
                </h4>
                <div className="flex items-center gap-2 ml-2 shrink-0">
                  <span className={`font-mono text-xs ${statusColors[task.status] || 'text-text-muted'}`}>
                    {statusLabels[task.status] || task.status}
                  </span>
                </div>
              </div>

              <div className="flex items-center gap-2 mb-3 font-mono text-xs text-text-muted">
                <span>{task.size || 'Unknown size'}</span>
                {task.format && (
                  <>
                    <span>•</span>
                    <span className="text-neon-blue">{task.format}</span>
                  </>
                )}
                {task.passwordUsed && (
                  <>
                    <span>•</span>
                    <span className="text-neon-orange flex items-center gap-1">
                      <Lock className="w-3 h-3" /> {task.passwordUsed}
                    </span>
                  </>
                )}
              </div>

              {task.status === 'extracting' && (
                <DotProgress progress={task.progress} hdrEnabled={hdrEnabled} />
              )}

              {task.status === 'completed' && (
                <div className="flex items-center gap-2 font-mono text-xs text-text-muted">
                  <CheckCircle className="w-3 h-3 text-neon-green" />
                  <span>Extracted to: {task.targetDir}</span>
                  <span>•</span>
                  <span>{task.files.length} files</span>
                </div>
              )}

              {task.status === 'error' && task.errorMessage && (
                <div className="flex items-center gap-2 font-mono text-xs text-neon-red">
                  <AlertCircle className="w-3 h-3" />
                  <span>{task.errorMessage}</span>
                </div>
              )}
            </div>

            <div className="flex items-center gap-2">
              {task.status === 'extracting' && (
                <button className="w-8 h-8 rounded-lg bg-bg-tertiary border border-border-subtle flex items-center justify-center hover:border-neon-orange hover:text-neon-orange transition-all">
                  <Pause className="w-4 h-4" />
                </button>
              )}
              <button
                className="w-8 h-8 rounded-lg bg-bg-tertiary border border-border-subtle flex items-center justify-center hover:border-neon-red hover:text-neon-red transition-all"
                onClick={() => removeTask(task.id)}
              >
                <X className="w-4 h-4" />
              </button>
            </div>
          </div>
        </div>
      ))}
    </div>
  )
}