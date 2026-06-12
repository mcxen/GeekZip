import { X, Moon, Languages, Zap, Eye } from 'lucide-react'
import { useAppStore } from '../stores/appStore'

function Toggle({ checked, onChange, hdrGlow }: { checked: boolean; onChange: (v: boolean) => void; hdrGlow?: boolean }) {
  return (
    <button
      onClick={() => onChange(!checked)}
      className={`relative w-10 h-5 rounded-full transition-all duration-300 ${
        checked ? 'bg-neon-green' : 'bg-bg-active'
      } ${checked && hdrGlow ? 'shadow-[0_0_10px_rgba(0,230,118,0.5)]' : ''}`}
    >
      <div className={`absolute top-0.5 w-4 h-4 rounded-full bg-white transition-all duration-300 ${
        checked ? 'left-[22px]' : 'left-0.5'
      }`} />
    </button>
  )
}

function SliderInput({ label, value, onChange, min, max, step, unit }: {
  label: string; value: number; onChange: (v: number) => void
  min: number; max: number; step: number; unit?: string
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="font-mono text-sm text-text-secondary">{label}</span>
      <div className="flex items-center gap-2">
        <input
          type="range" min={min} max={max} step={step}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          className="w-28 accent-[#00E676]"
        />
        <span className="font-mono text-xs text-neon-green w-16 text-right">
          {value}{unit || ''}
        </span>
      </div>
    </div>
  )
}

function SelectInput({ label, value, options, onChange }: {
  label: string; value: string; options: { value: string; label: string }[]; onChange: (v: string) => void
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="font-mono text-sm text-text-secondary">{label}</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="bg-bg-tertiary border border-border-default rounded-md px-3 py-1.5 font-mono text-sm text-text-primary focus:border-neon-green focus:outline-none"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>{o.label}</option>
        ))}
      </select>
    </div>
  )
}

export default function SettingsPanel() {
  const settings = useAppStore((s) => s.settings)
  const setSettings = useAppStore((s) => s.setSettings)
  const setShowSettings = useAppStore((s) => s.setShowSettings)
  const hdr = settings.hdrEnabled

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={() => setShowSettings(false)}>
      <div className={`w-[680px] max-h-[80vh] bg-bg-secondary border rounded-xl overflow-hidden ${
        hdr ? 'border-neon-green/30 shadow-[0_0_40px_rgba(0,230,118,0.15)]' : 'border-border-default'
      }`} onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border-subtle">
          <div className="flex items-center gap-3">
            <Zap className={`w-5 h-5 ${hdr ? 'text-neon-green' : 'text-text-muted'}`}
              style={hdr ? { filter: 'drop-shadow(0 0 6px rgba(0,230,118,0.6))' } : {}} />
            <h2 className="font-mono text-lg font-bold text-text-primary">Settings</h2>
          </div>
          <button onClick={() => setShowSettings(false)}
            className="w-8 h-8 rounded-lg bg-bg-tertiary border border-border-subtle flex items-center justify-center hover:border-neon-red hover:text-neon-red transition-all">
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="px-6 py-4 overflow-y-auto max-h-[calc(80vh-56px)] space-y-6">
          {/* HDR / Visual */}
          <section>
            <h3 className="font-mono text-xs text-neon-green uppercase tracking-wider mb-3">Visual</h3>
            <div className="space-y-3 bg-bg-tertiary/50 rounded-lg p-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Eye className="w-4 h-4 text-text-muted" />
                  <span className="font-mono text-sm text-text-secondary">HDR Bloom Effects</span>
                </div>
                <Toggle checked={settings.hdrEnabled} onChange={(v) => setSettings({ hdrEnabled: v })} hdrGlow />
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Moon className="w-4 h-4 text-text-muted" />
                  <span className="font-mono text-sm text-text-secondary">Theme</span>
                </div>
                <SelectInput
                  label="" value={settings.theme}
                  options={[{ value: 'dark', label: 'Dark' }, { value: 'light', label: 'Light' }]}
                  onChange={(v) => setSettings({ theme: v })}
                />
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Languages className="w-4 h-4 text-text-muted" />
                  <span className="font-mono text-sm text-text-secondary">Language</span>
                </div>
                <SelectInput
                  label="" value={settings.language}
                  options={[{ value: 'zh', label: '中文' }, { value: 'en', label: 'English' }]}
                  onChange={(v) => setSettings({ language: v })}
                />
              </div>
            </div>
          </section>

          {/* Extract */}
          <section>
            <h3 className="font-mono text-xs text-neon-green uppercase tracking-wider mb-3">Extract</h3>
            <div className="space-y-3 bg-bg-tertiary/50 rounded-lg p-4">
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Default Extract Path</span>
                <SelectInput
                  label="" value={settings.defaultExtractPath}
                  options={[
                    { value: 'current', label: 'Current Directory' },
                    { value: 'subfolder', label: 'Same Name Folder' },
                    { value: 'custom', label: 'Custom...' },
                  ]}
                  onChange={(v) => setSettings({ defaultExtractPath: v })}
                />
              </div>
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Overwrite Policy</span>
                <SelectInput
                  label="" value={settings.defaultOverwrite}
                  options={[
                    { value: 'rename', label: 'Auto Rename' },
                    { value: 'overwrite', label: 'Overwrite' },
                    { value: 'skip', label: 'Skip' },
                  ]}
                  onChange={(v) => setSettings({ defaultOverwrite: v })}
                />
              </div>
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Create Subfolder</span>
                <Toggle checked={settings.defaultCreateSubfolder} onChange={(v) => setSettings({ defaultCreateSubfolder: v })} hdrGlow={hdr} />
              </div>
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Open After Extract</span>
                <Toggle checked={settings.defaultOpenAfterExtract} onChange={(v) => setSettings({ defaultOpenAfterExtract: v })} hdrGlow={hdr} />
              </div>
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Delete After Extract</span>
                <Toggle checked={settings.defaultDeleteAfterExtract} onChange={(v) => setSettings({ defaultDeleteAfterExtract: v })} hdrGlow={hdr} />
              </div>
            </div>
          </section>

          {/* Recursive & Safety */}
          <section>
            <h3 className="font-mono text-xs text-neon-green uppercase tracking-wider mb-3">Recursive & Safety</h3>
            <div className="space-y-3 bg-bg-tertiary/50 rounded-lg p-4">
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Recursive Extract</span>
                <Toggle checked={settings.recursiveEnabled} onChange={(v) => setSettings({ recursiveEnabled: v })} hdrGlow={hdr} />
              </div>
              <SliderInput
                label="Max Depth" value={settings.recursiveMaxDepth}
                min={1} max={20} step={1}
                onChange={(v) => setSettings({ recursiveMaxDepth: v })}
              />
              <SliderInput
                label="Single File Limit" value={settings.singleFileSizeLimit}
                min={1} max={100} step={1} unit=" GB"
                onChange={(v) => setSettings({ singleFileSizeLimit: v })}
              />
              <SliderInput
                label="Total Extract Limit" value={settings.totalExtractSizeLimit}
                min={1} max={200} step={1} unit=" GB"
                onChange={(v) => setSettings({ totalExtractSizeLimit: v })}
              />
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Delete Intermediate Archives</span>
                <Toggle checked={settings.defaultDeleteIntermediate} onChange={(v) => setSettings({ defaultDeleteIntermediate: v })} hdrGlow={hdr} />
              </div>
            </div>
          </section>

          {/* Password */}
          <section>
            <h3 className="font-mono text-xs text-neon-green uppercase tracking-wider mb-3">Password</h3>
            <div className="space-y-3 bg-bg-tertiary/50 rounded-lg p-4">
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Auto-Save Successful Passwords</span>
                <Toggle checked={settings.autoSavePasswords} onChange={(v) => setSettings({ autoSavePasswords: v })} hdrGlow={hdr} />
              </div>
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Use Built-in Common Passwords</span>
                <Toggle checked={settings.useBuiltinPasswords} onChange={(v) => setSettings({ useBuiltinPasswords: v })} hdrGlow={hdr} />
              </div>
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Use Password Dictionary</span>
                <Toggle checked={settings.usePasswordDictionary} onChange={(v) => setSettings({ usePasswordDictionary: v })} hdrGlow={hdr} />
              </div>
              <SliderInput
                label="Password Timeout" value={settings.passwordTimeoutSeconds}
                min={5} max={120} step={5} unit="s"
                onChange={(v) => setSettings({ passwordTimeoutSeconds: v })}
              />
            </div>
          </section>

          {/* Performance */}
          <section>
            <h3 className="font-mono text-xs text-neon-green uppercase tracking-wider mb-3">Performance</h3>
            <div className="space-y-3 bg-bg-tertiary/50 rounded-lg p-4">
              <SliderInput
                label="Max Concurrent Tasks" value={settings.maxConcurrentTasks}
                min={1} max={16} step={1}
                onChange={(v) => setSettings({ maxConcurrentTasks: v })}
              />
              <SliderInput
                label="Max Threads" value={settings.maxThreads}
                min={1} max={32} step={1}
                onChange={(v) => setSettings({ maxThreads: v })}
              />
            </div>
          </section>

          {/* Notifications */}
          <section>
            <h3 className="font-mono text-xs text-neon-green uppercase tracking-wider mb-3">Notifications</h3>
            <div className="space-y-3 bg-bg-tertiary/50 rounded-lg p-4">
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Completion Notification</span>
                <Toggle checked={settings.showCompletionNotification} onChange={(v) => setSettings({ showCompletionNotification: v })} hdrGlow={hdr} />
              </div>
              <div className="flex items-center justify-between">
                <span className="font-mono text-sm text-text-secondary">Error Notification</span>
                <Toggle checked={settings.showErrorNotification} onChange={(v) => setSettings({ showErrorNotification: v })} hdrGlow={hdr} />
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  )
}