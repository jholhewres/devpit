import { useCallback, useEffect, useRef, useState } from 'react'

import { ask, commands } from './live'

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
}

/* The flat index behind name search: one fetch per project, not one per
   keystroke. `ensure` is a no-op once the project is in the cache, so a
   caller can call it on every change and still fetch only once. */
export function useFileIndex(projectId: string | null): UseFileIndex {
  const cache = useRef(new Map<string, Cached>())
  const [entry, setEntry] = useState<Cached | null>(null)
  const [loading, setLoading] = useState(false)

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
    const drop = (): void => {
      if (!projectId) return
      cache.current.delete(projectId)
      setEntry(null)
    }
    const onFocus = (): void => drop()
    const onVisible = (): void => {
      if (document.visibilityState === 'visible') drop()
    }
    window.addEventListener('focus', onFocus)
    document.addEventListener('visibilitychange', onVisible)
    return () => {
      window.removeEventListener('focus', onFocus)
      document.removeEventListener('visibilitychange', onVisible)
    }
  }, [projectId])

  const ensure = useCallback((): void => {
    if (!projectId || cache.current.has(projectId)) return
    setLoading(true)
    void ask(() => commands.projectFiles(projectId, null)).then((asked) => {
      const found: Cached = { paths: asked.data?.paths ?? [], partial: asked.data?.partial ?? false }
      cache.current.set(projectId, found)
      setEntry(found)
      setLoading(false)
    })
  }, [projectId])

  return { paths: entry?.paths ?? null, partial: entry?.partial ?? false, loading, ensure }
}
