import { useEffect, useState, useRef, useCallback } from 'react'

interface SystemStats {
  cpu_percent: number
  mem_total_mb: number
  mem_used_mb: number
  mem_percent: number
  threads: number
  cpu_history: number[]
  mem_history: number[]
}

function Sparkline({ data, color, width = 48, height = 16 }: {
  data: number[], color: string, width?: number, height?: number
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  const draw = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas || data.length < 2) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const dpr = window.devicePixelRatio || 1
    canvas.width = width * dpr
    canvas.height = height * dpr
    ctx.scale(dpr, dpr)

    ctx.clearRect(0, 0, width, height)

    const max = Math.max(100, ...data)
    const min = 0
    const range = max - min || 1

    const stepX = width / (data.length - 1)

    // Compute control points for smooth bezier
    const points = data.map((v, i) => ({
      x: i * stepX,
      y: height - ((v - min) / range) * height * 0.85 - height * 0.05,
    }))

    // Fill gradient below curve
    ctx.beginPath()
    ctx.moveTo(points[0].x, height)
    ctx.lineTo(points[0].x, points[0].y)

    for (let i = 1; i < points.length; i++) {
      const prev = points[i - 1]
      const curr = points[i]
      const cpx = (prev.x + curr.x) / 2
      ctx.bezierCurveTo(cpx, prev.y, cpx, curr.y, curr.x, curr.y)
    }

    ctx.lineTo(points[points.length - 1].x, height)
    ctx.closePath()

    const gradient = ctx.createLinearGradient(0, 0, 0, height)
    gradient.addColorStop(0, color === '#00E676'
      ? 'rgba(0, 230, 118, 0.25)'
      : 'rgba(41, 98, 255, 0.25)')
    gradient.addColorStop(1, 'rgba(0, 0, 0, 0)')
    ctx.fillStyle = gradient
    ctx.fill()

    // Draw the curve line
    ctx.beginPath()
    ctx.moveTo(points[0].x, points[0].y)
    for (let i = 1; i < points.length; i++) {
      const prev = points[i - 1]
      const curr = points[i]
      const cpx = (prev.x + curr.x) / 2
      ctx.bezierCurveTo(cpx, prev.y, cpx, curr.y, curr.x, curr.y)
    }

    ctx.strokeStyle = color
    ctx.lineWidth = 1.5
    ctx.lineJoin = 'round'
    ctx.lineCap = 'round'

    if (color === '#00E676') {
      ctx.shadowColor = 'rgba(0, 230, 118, 0.6)'
      ctx.shadowBlur = 4
    } else {
      ctx.shadowColor = 'rgba(41, 98, 255, 0.6)'
      ctx.shadowBlur = 4
    }
    ctx.stroke()
    ctx.shadowBlur = 0

    // Current value dot
    const last = points[points.length - 1]
    ctx.beginPath()
    ctx.arc(last.x, last.y, 2.5, 0, Math.PI * 2)
    ctx.fillStyle = color
    ctx.fill()
    if (color === '#00E676') {
      ctx.shadowColor = 'rgba(0, 230, 118, 0.8)'
      ctx.shadowBlur = 6
    } else {
      ctx.shadowColor = 'rgba(41, 98, 255, 0.8)'
      ctx.shadowBlur = 6
    }
    ctx.fill()
    ctx.shadowBlur = 0

  }, [data, color, width, height])

  useEffect(() => {
    draw()
  }, [draw])

  return (
    <canvas
      ref={canvasRef}
      style={{ width: `${width}px`, height: `${height}px` }}
      className="shrink-0"
    />
  )
}

function formatMem(mb: number): string {
  if (mb < 1024) return `${mb}MB`
  return `${(mb / 1024).toFixed(1)}GB`
}

export default function StatusBar() {
  const [stats, setStats] = useState<SystemStats | null>(null)
  const [cpuHistory, setCpuHistory] = useState<number[]>([])
  const [memHistory, setMemHistory] = useState<number[]>([])

  useEffect(() => {
    let mounted = true
    const fetchStats = async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core')
        const s = await invoke('get_system_stats') as SystemStats
        if (!mounted) return
        setStats(s)
        setCpuHistory(prev => [...prev.slice(-59), s.cpu_percent])
        setMemHistory(prev => [...prev.slice(-59), s.mem_percent])
      } catch {
        if (!mounted) return
        setStats({
          cpu_percent: 0, mem_total_mb: 16384, mem_used_mb: 6144,
          mem_percent: 37, threads: 8, cpu_history: [], mem_history: [],
        })
      }
    }
    fetchStats()
    const interval = setInterval(fetchStats, 1000)
    return () => { mounted = false; clearInterval(interval) }
  }, [])

  const cpu = stats?.cpu_percent ?? 0
  const memUsed = stats?.mem_used_mb ?? 0
  const memTotal = stats?.mem_total_mb ?? 0
  const threads = stats?.threads ?? 0

  const items = [
    { label: 'CPU', value: `${cpu}%`, color: '#00E676', history: cpuHistory },
    { label: 'MEM', value: `${formatMem(memUsed)}`, sub: `/ ${formatMem(memTotal)}`, color: '#2962FF', history: memHistory },
    { label: 'THREADS', value: `${threads}`, color: '#FF6D00', history: [] },
  ]

  return (
    <div className="h-8 bg-bg-secondary border-t border-border-subtle flex items-center px-4 gap-5 shrink-0">
      {items.map((stat) => (
        <div key={stat.label} className="flex items-center gap-2">
          <span className="font-mono text-[10px] text-text-muted uppercase tracking-wider">
            {stat.label}
          </span>
          <span className="font-mono text-xs" style={{ color: stat.color }}>
            {stat.value}
          </span>
          {stat.sub && (
            <span className="font-mono text-[10px] text-text-muted">
              {stat.sub}
            </span>
          )}
          {stat.history.length >= 2 && (
            <Sparkline data={stat.history} color={stat.color} width={48} height={16} />
          )}
        </div>
      ))}

      <div className="flex-1" />

      <div className="flex items-center gap-2">
        <div className="w-2 h-2 rounded-full bg-neon-green animate-pulse" />
        <span className="font-mono text-[10px] text-neon-green tracking-wider uppercase">
          All Systems Operational
        </span>
      </div>
    </div>
  )
}