import { useCallback, useEffect, useRef, useState } from 'react'

import type { FileContents, WorkspaceListing } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * Reading the devpit workspace: one folder, and one file out of it.
 *
 * Both halves in one file because they are one panel — the folder says what
 * there is and the file says what one of them holds, and a reader looking for
 * "how does the Files panel get its data" should find the answer once.
 */

export interface Folder {
  readonly listing: WorkspaceListing | null
  readonly error: string | null
  /* An empty listing is ambiguous — nothing in the folder, or not back yet —
     and a panel that cannot tell them apart draws a real folder as empty for
     as long as the read takes. */
  readonly loading: boolean
  /** Lists this folder. `null` means "wherever this project's files are". */
  go: (path: string | null) => void
  reload: () => void
}

/* A response belongs to the generation that asked for it. Two reads can be in
   flight at once — a click into a folder while a refresh is landing — and
   nothing says the newer one resolves last. */
const stale = (mine: number, now: number): boolean => mine !== now

export function useWorkspaceFolder(projectId: string | null): Folder {
  /* The path asked for, which is not the path listed: `null` is a question
     only the backend can answer, and it answers it in `listing.path`. */
  const [wanted, setWanted] = useState<string | null>(null)
  const [listing, setListing] = useState<WorkspaceListing | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const generation = useRef(0)
  const [again, setAgain] = useState(0)

  useEffect(() => {
    const mine = ++generation.current
    setLoading(true)
    void ask(() => commands.workspaceList(projectId, wanted)).then((answer) => {
      if (stale(mine, generation.current)) return
      setError(answer.error)
      if (answer.data) setListing(answer.data)
      setLoading(false)
    })
  }, [projectId, wanted, again])

  /* Back to the project's own folder when the project changes: the previous
     project's worktree path is a folder this one has nothing to do with. */
  useEffect(() => setWanted(null), [projectId])

  return {
    listing,
    error,
    loading,
    go: useCallback((path: string | null) => setWanted(path), []),
    reload: useCallback(() => setAgain((count) => count + 1), []),
  }
}

export interface Preview {
  readonly file: FileContents | null
  readonly error: string | null
  readonly loading: boolean
}

export function useWorkspaceFile(path: string | null): Preview {
  const [file, setFile] = useState<FileContents | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const generation = useRef(0)

  useEffect(() => {
    const mine = ++generation.current
    if (!path) {
      setFile(null)
      setError(null)
      setLoading(false)
      return
    }
    setLoading(true)
    void ask(() => commands.workspaceFile(path)).then((answer) => {
      if (stale(mine, generation.current)) return
      setFile(answer.data ?? null)
      setError(answer.error)
      setLoading(false)
    })
  }, [path])

  return { file, error, loading }
}
