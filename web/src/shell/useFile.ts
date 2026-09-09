import { useCallback, useEffect, useState } from 'react'

import type { FileContents } from '../gen/bindings'
import { ask, commands } from './live'
import { useShell } from './useShell'

export interface Editing {
  readonly file: FileContents | null
  /** What is in the editor, which is the file until someone types. */
  readonly text: string
  readonly dirty: boolean
  readonly error: string | null
  readonly saving: boolean
  /** Set when a save was refused because the file moved on under it. */
  readonly clash: string | null
  change: (text: string) => void
  save: () => void
  /** Throws away the edit and takes what is on disk. */
  reload: () => void
  /** Saves over what is on disk, losing the other change. Asked for, never
      chosen by default. */
  overwrite: () => void
}

export function useFile(path: string | null): Editing {
  const { project } = useShell()
  const [file, setFile] = useState<FileContents | null>(null)
  const [text, setText] = useState('')
  const [dirty, setDirty] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const [clash, setClash] = useState<string | null>(null)

  const load = useCallback(() => {
    if (!project || !path) return
    void ask(() => commands.fileRead(project.id, null, path)).then((answer) => {
      setFile(answer.data ?? null)
      setText(answer.data?.text ?? '')
      setDirty(false)
      setClash(null)
      setError(answer.error)
    })
  }, [project, path])

  useEffect(load, [load])

  const write = useCallback(
    (readAt: number) => {
      if (!project || !path) return
      setSaving(true)
      void ask(() => commands.fileWrite(project.id, null, path, text, readAt))
        .then((answer) => {
          if (answer.data) {
            /* The mtime the save produced becomes the one the next save is
               built on, or every save after the first reads as stale. */
            setFile((was) => (was ? { ...was, readAt: answer.data!.readAt } : was))
            setDirty(false)
            setClash(null)
            setError(null)
          } else {
            setClash(answer.error)
          }
        })
        .finally(() => setSaving(false))
    },
    [project, path, text],
  )

  return {
    file,
    text,
    dirty,
    error,
    saving,
    clash,
    change: (next: string) => {
      setText(next)
      setDirty(next !== (file?.text ?? ''))
    },
    save: () => write(file?.readAt ?? 0),
    reload: load,
    /* Zero means "built on no read", which `is_stale` lets through. That is
       the overwrite, said in the one word the backend already understands. */
    overwrite: () => write(0),
  }
}
