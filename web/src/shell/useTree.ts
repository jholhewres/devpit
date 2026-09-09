import { useCallback, useEffect, useState } from 'react'

import type { Change, FileNode } from '../gen/bindings'
import { ask, commands } from './live'

export interface UseTree {
  readonly nodes: readonly FileNode[]
  readonly changes: readonly Change[]
  readonly totals: { added: number; removed: number }
  readonly error: string | null
  reload: () => void
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

  const reload = useCallback(() => {
    if (!projectId) {
      setNodes([])
      setChanges([])
      return
    }
    void ask(() => commands.projectTree(projectId, null, '')).then((asked) => {
      setError(asked.error)
      if (asked.data) setNodes(asked.data.nodes)
    })
    void ask(() => commands.projectChanges(projectId, null)).then((asked) => {
      if (asked.data) {
        setChanges(asked.data.changes)
        setTotals({ added: asked.data.added, removed: asked.data.removed })
      }
    })
  }, [projectId])

  useEffect(reload, [reload])

  return { nodes, changes, totals, error, reload }
}
