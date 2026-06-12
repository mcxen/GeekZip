import { useState, useCallback } from 'react'
import { useAppStore } from '../stores/appStore'

let taskId = 0

export function useExtract() {
  const addTask = useAppStore((s) => s.addTask)
  const updateTask = useAppStore((s) => s.updateTask)
  const [loading, setLoading] = useState(false)

  const extract = useCallback(async (filePath: string, options?: {
    targetDir?: string
    password?: string
    recursive?: boolean
    deleteAfter?: boolean
  }) => {
    const id = `task-${++taskId}`
    const name = filePath.split('/').pop() || filePath

    addTask({
      id,
      source: filePath,
      name,
      size: 0,
      format: '',
      status: 'analyzing',
      progress: 0,
      speed: '',
      eta: '',
      files: [],
      targetDir: '',
      createdAt: Date.now(),
    })

    setLoading(true)
    try {
      updateTask(id, { status: 'extracting', progress: 10 })

      const { invoke } = await import('@tauri-apps/api/core')

      const result = await invoke('extract_smart', {
        path: filePath,
        targetDir: options?.targetDir || null,
        recursive: options?.recursive ?? false,
        maxDepth: 10,
        password: options?.password || null,
        deleteAfter: options?.deleteAfter ?? false,
      }) as any

      updateTask(id, {
        status: 'completed',
        progress: 100,
        format: result.format || '',
        files: result.files || [],
        targetDir: result.target_dir || '',
        speed: `${(result.elapsed_ms || 0)}ms`,
      })

      return result
    } catch (err: any) {
      updateTask(id, {
        status: 'error',
        errorMessage: String(err),
      })
      throw err
    } finally {
      setLoading(false)
    }
  }, [addTask, updateTask])

  return { extract, loading }
}