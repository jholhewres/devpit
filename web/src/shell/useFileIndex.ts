import { useCallback, useEffect, useRef, useState } from 'react'

import { ask, commands } from './live'
import { CHANGED } from './useTree'

export interface UseFileIndex {
  readonly paths: readonly string[] | null
  /* True once the walk that built this index stopped at its ceiling — the
     list may be missing files, and a caller that hides that turns a missing
     file into a false "not found". */
  readonly partial: boolean
  readonly loading: boolean
  ensure: () => void
}

interface Cached {
  readonly paths: readonly string[]
  readonly partial: boolean
  /** When it was walked: a focus drops only an index older than `FRESH`. */
  readonly at: number
}

/* Coming back to the window drops an index older than this. A focus is
   frequent and a walk is up to forty thousand files, so one that is seconds
   old is kept; what this app itself changes drops it at once (`CHANGED`). */
const FRESH = 30_000

/* The flat index behind name search: one fetch per project, not one per
   keystroke. `ensure` is a no-op once the project is in the cache, so a
   caller can call it on every change and still fetch only once. */
export function useFileIndex(projectId: string | null): UseFileIndex {
  const cache = useRef(new Map<string, Cached>())
  const [entry, setEntry] = useState<Cached | null>(null)
  /* Which projects have a walk on its way: loading is this project's, so a
     walk for the one open a moment ago cannot leave this one spinning. */
  const [walking, setWalking] = useState<ReadonlySet<string>>(() => new Set())
  const loading = projectId !== null && walking.has(projectId)
  /* Bumped by a change: a walk that started before it lists the files as they
     were, and is not kept. */
  const generation = useRef(new Map<string, number>())

  useEffect(() => {
    setEntry(projectId ? (cache.current.get(projectId) ?? null) : null)
  }, [projectId])

  /* This cache never expires on its own — `ensure` is a no-op once a project
     is in it — so a file created or deleted after the walk stays invisible
     to name search until something drops the entry. Reusing `useTree`'s two
     event triggers (not its generation counter: `ensure` never has two
     fetches for the same project in flight, since the cache only gains an
     entry after the one fetch resolves) is the cheap half of the fix — it
     costs nothing while idle and nothing while the project is not being
     searched, because dropping the entry does not itself refetch it.
     Stage/unstage/commit change git status, not the file set, so they are
     not triggers here; discard can delete an untracked file, but wiring
     that from `Changes.tsx` for one edge case was not worth the coupling —
     the next focus or visibility change clears it regardless. */
  useEffect(() => {
    const drop = (always: boolean): void => {
      if (!projectId) return
      const had = cache.current.get(projectId)
      if (!had || (!always && Date.now() - had.at < FRESH)) return
      cache.current.delete(projectId)
      setEntry(null)
    }
    const onFocus = (): void => drop(false)
    const onVisible = (): void => {
      if (document.visibilityState === 'visible') drop(false)
    }
    const onChanged = (): void => {
      if (projectId) generation.current.set(projectId, (generation.current.get(projectId) ?? 0) + 1)
      drop(true)
    }
    window.addEventListener('focus', onFocus)
    window.addEventListener(CHANGED, onChanged)
    document.addEventListener('visibilitychange', onVisible)
    return () => {
      window.removeEventListener('focus', onFocus)
      window.removeEventListener(CHANGED, onChanged)
      document.removeEventListener('visibilitychange', onVisible)
    }
  }, [projectId])

  /* The project asked for now: an answer for the one open a moment ago is
     cached under its own name and not shown in this one. */
  const current = useRef(projectId)
  current.current = projectId

  const ensure = useCallback((): void => {
    if (!projectId || cache.current.has(projectId) || walking.has(projectId)) return
    const started = generation.current.get(projectId) ?? 0
    setWalking((was) => new Set(was).add(projectId))
    void ask(() => commands.projectFiles(projectId, null)).then((asked) => {
      setWalking((was) => {
        const next = new Set(was)
        next.delete(projectId)
        return next
      })
      const found: Cached = { paths: asked.data?.paths ?? [], partial: asked.data?.partial ?? false, at: Date.now() }
      if ((generation.current.get(projectId) ?? 0) !== started) return
      cache.current.set(projectId, found)
      if (current.current !== projectId) return
      setEntry(found)
    })
  }, [projectId, walking])

  return { paths: entry?.paths ?? null, partial: entry?.partial ?? false, loading, ensure }
}
