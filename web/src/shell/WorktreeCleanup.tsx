import { useEffect, useState } from 'react'

import type { CleanupRun, SweptWhy, WorktreeCleanup as Cleanup } from '../gen/bindings'
import { bytes } from './disk'
import { ask, commands } from './live'
import { PrefRow } from './PrefRow'
import { PrefSwitch } from './PrefSwitch'
import { ago } from './usageFormat'

/*
 * Merged cards' worktrees put away: on its own every hour, or now. What the
 * last run removed, kept and freed is said under the button.
 */

const KEPT: Record<SweptWhy, string> = {
  landed: 'merged',
  uncommitted: 'uncommitted work',
  running: 'a session is running in it',
  failed: 'git refused',
}

function lastRun(run: CleanupRun, nowMs: number): string {
  return `Last run ${ago(run.at, nowMs)}: removed ${run.removed.length}, kept ${run.kept.length}, freed ${bytes(run.freedBytes) ?? '0 B'}.`
}

export function WorktreeCleanup(): React.JSX.Element {
  const [cleanup, setCleanup] = useState<Cleanup | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  /* When the answer came, so "ago" is read from then rather than during render. */
  const [heardAt, setHeardAt] = useState(0)
  const take = (data: Cleanup | null): void => {
    setCleanup(data)
    setHeardAt(Date.now())
  }

  useEffect(() => {
    void ask(() => commands.worktreeCleanupRead()).then((answer) => take(answer.data ?? null))
  }, [])

  const flip = (): void => {
    if (!cleanup) return
    void ask(() => commands.worktreeCleanupWrite(!cleanup.deleteMerged)).then((answer) => answer.data && take(answer.data))
  }

  const now = (): void => {
    setBusy(true)
    void ask(() => commands.worktreeCleanupNow()).then((answer) => {
      setBusy(false)
      setError(answer.error)
      if (answer.data) take(answer.data)
    })
  }

  const last = cleanup?.last ?? null
  return (
    <>
      <PrefSwitch
        on={cleanup?.deleteMerged ?? true}
        onFlip={flip}
        title="Delete merged worktrees"
        said="Once a card's commits are in the default branch, its worktree, build cache and local branch go. One with uncommitted work, or a session running in it, is kept."
      />
      <PrefRow title="Clean up now" said={last ? lastRun(last, heardAt) : 'Not run yet.'}>
        <button className="btn" disabled={busy} onClick={now}>
          {busy ? 'Deleting…' : 'Delete now'}
        </button>
      </PrefRow>
      {error && <p className="pref__d">{error}</p>}
      {last?.kept.map((row) => (
        <p className="pref__d" key={row.cardId}>
          Kept {row.title}: {KEPT[row.why]}
          {row.detail ? ` (${row.detail})` : ''}
        </p>
      ))}
    </>
  )
}
