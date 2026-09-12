import { useCallback, useEffect, useRef, useState } from 'react'

import type { Change, FileNode } from '../gen/bindings'
import { ask, commands } from './live'

export interface UseTree {
  readonly nodes: readonly FileNode[]
  readonly changes: readonly Change[]
  readonly totals: { added: number; removed: number }
  readonly error: string | null
  /* Bumped by every reload. `Tree.tsx` only ever has the ROOT level from
     `nodes` — a folder's own children come from a separate, lazy fetch this
     hook does not make — so a row watches this to know its cached children
     need the same treatment a reload just gave the root. */
  readonly version: number
  /* True from the moment a reload starts until the tree fetch answers. An
     empty `nodes` is ambiguous — no project, or just not back yet — and a
     caller that cannot tell the two apart draws a real project as an empty
     one for as long as the fetch takes. */
  readonly loading: boolean
  reload: () => void
}

/* What a mutation outside this hook announces when it has changed the tree.
   An event, not a callback threaded through four components: the panel that
   owns the tree and the menu that creates a file are siblings, and the menu
   has no business holding a reference to the panel's reload. */
const CHANGED = 'devpit:tree-changed'

export const changed = (): void => {
  window.dispatchEvent(new Event(CHANGED))
}

/* A fetch's response belongs to the generation that asked for it. Two fetches
   can be in flight at once — a manual Refresh while a focus-triggered reload
   is still landing — and nothing guarantees the newer one resolves last, so
   whichever response arrives is not necessarily the current answer. Pulled
   out so a test can hand it two numbers instead of racing two promises. */
export function isStale(responseGeneration: number, currentGeneration: number): boolean {
  return responseGeneration !== currentGeneration
}

/* The tree and the changed-file list are the same question asked twice, so
   one hook fetches both and one Refresh answers for both. */
export function useTree(projectId: string | null): UseTree {
  const [nodes, setNodes] = useState<readonly FileNode[]>([])
  const [changes, setChanges] = useState<readonly Change[]>([])
  /* The totals come from the server; the screen never adds up a list it may
     have received truncated. */
  const [totals, setTotals] = useState({ added: 0, removed: 0 })
  const [error, setError] = useState<string | null>(null)
  /* A ref, not state: the response handler below must read whatever the
     LATEST reload set, not the value closed over when its own fetch started. */
  const generation = useRef(0)
  const [version, setVersion] = useState(0)
  /* Starts true only when there is a project to fetch — otherwise the very
     first render would flash a skeleton for a window that opens on no
     project at all. */
  const [loading, setLoading] = useState(projectId !== null)

  const reload = useCallback(() => {
    const mine = ++generation.current
    setVersion(mine)
    setLoading(true)
    if (!projectId) {
      setNodes([])
      setChanges([])
      setLoading(false)
      return
    }
    /* A fresh array, not a fresh identity: `Tree.tsx` keys each row by
       `node.path` and keeps a folder's open/closed state in that row's own
       component instance, so replacing `nodes` here does not remount a row
       whose path is unchanged and does not close it. The naive fix — keying
       rows on the reload's generation, or rebuilding the tree from scratch —
       would remount every row and collapse everything the reader opened. */
    void ask(() => commands.projectTree(projectId, null, '')).then((asked) => {
      if (isStale(mine, generation.current)) return
      setError(asked.error)
      if (asked.data) setNodes(asked.data.nodes)
      setLoading(false)
    })
    void ask(() => commands.projectChanges(projectId, null)).then((asked) => {
      if (isStale(mine, generation.current)) return
      if (asked.data) {
        setChanges(asked.data.changes)
        setTotals({ added: asked.data.added, removed: asked.data.removed })
      }
    })
  }, [projectId])

  useEffect(reload, [reload])

  /* Disk state moves without any click in this window — another worktree,
     another process, an agent. Reacting to the moments that plausibly mean
     that beats polling, which this codebase treats as work that keeps itself
     going for no reason: the window regaining focus and the tab becoming
     visible again. Both are event listeners registered from an effect, never
     called from render — a render path may not reach I/O, only schedule it. */
  useEffect(() => {
    const onFocus = (): void => reload()
    const onVisible = (): void => {
      if (document.visibilityState === 'visible') reload()
    }
    const onChanged = (): void => reload()
    window.addEventListener('focus', onFocus)
    window.addEventListener(CHANGED, onChanged)
    document.addEventListener('visibilitychange', onVisible)
    return () => {
      window.removeEventListener('focus', onFocus)
      window.removeEventListener(CHANGED, onChanged)
      document.removeEventListener('visibilitychange', onVisible)
    }
  }, [reload])

  return { nodes, changes, totals, error, version, loading, reload }
}
