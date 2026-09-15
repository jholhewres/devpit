import { useCallback, useEffect, useRef, useState } from 'react'

import { ask, commands } from '../../shell/live'
import { useShell } from '../../shell/useShell'
import { drawingFileValid } from './drawingFile'
import { drawingName, PLUGIN_ID, SAVE_AFTER_MS, stemOf } from './drawings'

/*
 * One drawing file: read once, written a moment after the last change.
 *
 * Every write carries the modified time the last read or write returned, so a
 * file someone else changed is refused by the backend instead of overwritten
 * — and the refusal waits for a person, it is never retried on its own.
 */

export interface DrawingFile {
  /** What the canvas starts from; null until read, and for a file it cannot open. */
  readonly text: string | null
  /** Bumped on every read, so a reload remounts the canvas on what is on disk. */
  readonly generation: number
  readonly conflict: boolean
  readonly problem: string | null
  change: (serialized: string) => void
  reload: () => void
  keepMine: () => void
  /** The new file name, or null when nothing was renamed. */
  rename: (typed: string) => Promise<string | null>
}

export function useDrawing(name: string): DrawingFile {
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
    void ask(() => commands.pluginDataRead(projectId, PLUGIN_ID, name)).then((answer) => {
      if (!answer.data) return setProblem(answer.error)
      /* A canvas opened on a file it cannot parse would save an empty scene over it. */
      if (!drawingFileValid(answer.data.text)) {
        return setProblem(
          `${stemOf(name)} cannot be opened here, so it is left as it is: it is not an Excalidraw drawing, or it embeds web content.`,
        )
      }
      modified.current = answer.data.modified
      saved.current = answer.data.text
      pending.current = null
      refused.current = false
      setConflict(false)
      setProblem(null)
      setText(answer.data.text)
      setGeneration((was) => was + 1)
    })
  }, [projectId, name])

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
        commands.pluginDataWrite(projectId, PLUGIN_ID, name, next, modified.current),
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
  }, [projectId, name])

  const change = useCallback(
    (serialized: string) => {
      /* The canvas reports selection and scrolling too; only a different file is a change. */
      if (serialized === (pending.current ?? saved.current)) return
      pending.current = serialized
      if (timer.current !== null) window.clearTimeout(timer.current)
      timer.current = window.setTimeout(flush, SAVE_AFTER_MS)
    },
    [flush],
  )

  /* A tab closed inside the wait still writes what was drawn. */
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
    void ask(() => commands.pluginDataRead(projectId, PLUGIN_ID, name)).then((answer) => {
      if (!answer.data && answer.code !== 'not_found') return setProblem(answer.error)
      modified.current = answer.data?.modified ?? null
      refused.current = false
      setConflict(false)
      flush()
    })
  }, [projectId, name, flush])

  const rename = useCallback(
    async (typed: string): Promise<string | null> => {
      const wanted = drawingName(typed)
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
      const made = await ask(() => commands.pluginDataWrite(projectId, PLUGIN_ID, wanted.name, body, null))
      if (!made.data) {
        setProblem(made.code === 'conflict' ? `${stemOf(wanted.name)} already exists.` : made.error)
        return null
      }
      const gone = await ask(() => commands.pluginDataDelete(projectId, PLUGIN_ID, name))
      if (gone.error) {
        setProblem(`Saved as ${stemOf(wanted.name)}, but ${stemOf(name)} could not be removed: ${gone.error}`)
        return null
      }
      return made.data.name
    },
    [projectId, name, flush],
  )

  return { text, generation, conflict, problem, change, reload, keepMine, rename }
}
