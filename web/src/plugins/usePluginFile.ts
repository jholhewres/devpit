import { useCallback, useEffect, useRef, useState } from 'react'

import { ask, commands } from '../shell/live'
import { useShell } from '../shell/useShell'
import { fileName, SAVE_AFTER_MS, stemOf, type FileKind } from './pluginFile'

/*
 * One Capability file: read once, written a moment after the last change.
 *
 * Every write carries the modified time the last read or write returned, so a
 * file someone else changed is refused by the backend instead of overwritten
 * — and the refusal waits for a person, it is never retried on its own.
 *
 * Generic over the Capability because the second one needed exactly this and
 * a copy of it would be a second set of bugs. What differs is the kind and,
 * for a format that can be malformed, a reader that says so before an editor
 * opens on it and saves something emptier over the top.
 */

export interface PluginFile {
  /** What the editor starts from; null until read, and for a file it cannot open. */
  readonly text: string | null
  /** Bumped on every read, so a reload remounts the editor on what is on disk. */
  readonly generation: number
  readonly conflict: boolean
  readonly problem: string | null
  change: (serialized: string) => void
  reload: () => void
  keepMine: () => void
  /** The new file name, or null when nothing was renamed. */
  rename: (typed: string) => Promise<string | null>
}

export function usePluginFile(
  kind: FileKind,
  name: string,
  /** Null when the text is fine, or the sentence to show instead of opening it. */
  refuse: (text: string) => string | null = () => null,
): PluginFile {
  const { project } = useShell()
  const projectId = project?.id ?? null
  const [text, setText] = useState<string | null>(null)
  const [generation, setGeneration] = useState(0)
  const [conflict, setConflict] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  /* Refs, because the timer and the write chain run after the render that set them. */
  const modified = useRef<number | null>(null)
  const saved = useRef<string | null>(null)
  const pending = useRef<string | null>(null)
  const refused = useRef(false)
  const timer = useRef<number | null>(null)
  const writes = useRef<Promise<void>>(Promise.resolve())

  const read = useCallback(() => {
    if (!projectId) return
    void ask(() => commands.pluginDataRead(projectId, kind.pluginId, name)).then((answer) => {
      if (!answer.data) return setProblem(answer.error)
      /* An editor opened on a file it cannot parse would save an empty one over it. */
      const why = refuse(answer.data.text)
      if (why) return setProblem(why)
      modified.current = answer.data.modified
      saved.current = answer.data.text
      pending.current = null
      refused.current = false
      setConflict(false)
      setProblem(null)
      setText(answer.data.text)
      setGeneration((was) => was + 1)
    })
    // `refuse` is a rule rather than state; taking it as a dependency would
    // re-read the file on every render that spells it inline.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, kind.pluginId, name])

  useEffect(read, [read])

  const flush = useCallback(() => {
    if (timer.current !== null) window.clearTimeout(timer.current)
    timer.current = null
    /* Chained, so each write carries the modified time the one before it returned. */
    writes.current = writes.current.then(async () => {
      const next = pending.current
      if (!projectId || next === null || refused.current) return
      pending.current = null
      const answer = await ask(() =>
        commands.pluginDataWrite(projectId, kind.pluginId, name, next, modified.current),
      )
      if (answer.data) {
        modified.current = answer.data.modified
        saved.current = next
        return setProblem(null)
      }
      pending.current ??= next
      if (answer.code === 'conflict') {
        refused.current = true
        setConflict(true)
      } else {
        setProblem(answer.error)
      }
    })
  }, [projectId, kind.pluginId, name])

  const change = useCallback(
    (serialized: string) => {
      /* An editor reports selection and scrolling too; only a different file
         is a change. */
      if (serialized === (pending.current ?? saved.current)) return
      pending.current = serialized
      if (timer.current !== null) window.clearTimeout(timer.current)
      timer.current = window.setTimeout(flush, SAVE_AFTER_MS)
    },
    [flush],
  )

  /* A tab closed inside the wait still writes what was typed. */
  useEffect(
    () => () => {
      if (timer.current !== null) flush()
    },
    [flush],
  )

  const reload = useCallback(() => {
    if (timer.current !== null) window.clearTimeout(timer.current)
    timer.current = null
    read()
  }, [read])

  const keepMine = useCallback(() => {
    if (!projectId) return
    /* Only the modified time comes from disk; what is written is what is on screen. */
    void ask(() => commands.pluginDataRead(projectId, kind.pluginId, name)).then((answer) => {
      if (!answer.data && answer.code !== 'not_found') return setProblem(answer.error)
      modified.current = answer.data?.modified ?? null
      refused.current = false
      setConflict(false)
      flush()
    })
  }, [projectId, kind.pluginId, name, flush])

  const rename = useCallback(
    async (typed: string): Promise<string | null> => {
      const wanted = fileName(kind, typed)
      if ('problem' in wanted) {
        setProblem(wanted.problem)
        return null
      }
      if (!projectId || wanted.name === name) return null
      /* What is waiting lands on the old file first, so the copy is the latest. */
      flush()
      await writes.current
      const body = pending.current ?? saved.current
      if (refused.current || body === null) return null
      const made = await ask(() =>
        commands.pluginDataWrite(projectId, kind.pluginId, wanted.name, body, null),
      )
      if (!made.data) {
        setProblem(
          made.code === 'conflict' ? `${stemOf(kind, wanted.name)} already exists.` : made.error,
        )
        return null
      }
      const gone = await ask(() => commands.pluginDataDelete(projectId, kind.pluginId, name))
      if (gone.error) {
        setProblem(
          `Saved as ${stemOf(kind, wanted.name)}, but ${stemOf(kind, name)} could not be removed: ${gone.error}`,
        )
        return null
      }
      return made.data.name
    },
    [projectId, kind, name, flush],
  )

  return { text, generation, conflict, problem, change, reload, keepMine, rename }
}
