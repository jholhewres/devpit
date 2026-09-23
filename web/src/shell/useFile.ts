import { useCallback, useEffect, useRef, useState } from 'react'

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
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const [clash, setClash] = useState<string | null>(null)

  /* Only the latest read lands: a slow read of the file open a moment ago
     must not fill the one open now. */
  const reading = useRef(0)
  const load = useCallback(() => {
    if (!project || !path) return
    const mine = ++reading.current
    /* A full path — one clicked in a terminal — is read where it is, if it is
       somewhere the app may open; a project path through the project. */
    const read = path.startsWith('/') ? () => commands.pathRead(path) : () => commands.fileRead(project.id, null, path)
    void ask(read).then((answer) => {
      if (mine !== reading.current) return
      setFile(answer.data ?? null)
      setText(answer.data?.text ?? '')
      setClash(null)
      setError(answer.error)
    })
  }, [project, path])

  useEffect(load, [load])

  const write = useCallback(
    (readAt: number) => {
      if (!project || !path) return
      setSaving(true)
      const saved = text
      void ask(() => commands.fileWrite(project.id, null, path, saved, readAt))
        .then((answer) => {
          if (answer.data) {
            /* The mtime the save produced becomes the one the next save is
               built on, or every save after the first reads as stale — and
               what was saved becomes what "changed" is measured against. */
            setFile((was) => (was ? { ...was, readAt: answer.data!.readAt, text: saved } : was))
            setClash(null)
            setError(null)
          } else if (answer.code === 'conflict') {
            setClash(answer.error)
          } else {
            /* A refusal that is not the file moving on — no permission, a
               full disk — is said as itself, not as a conflict to overwrite. */
            setError(answer.error)
          }
        })
        .finally(() => setSaving(false))
    },
    [project, path, text],
  )

  /* Worked out, not kept: typing while a save is on its way leaves the file
     unsaved, and a flag cleared by the save said otherwise — a tab that
     closed without asking and lost what was typed. */
  const dirty = file !== null && text !== (file.text ?? '')

  return {
    file,
    text,
    dirty,
    error,
    saving,
    clash,
    change: setText,
    save: () => write(file?.readAt ?? 0),
    reload: load,
    /* Zero means "built on no read", which `is_stale` lets through. That is
       the overwrite, said in the one word the backend already understands. */
    overwrite: () => write(0),
  }
}
