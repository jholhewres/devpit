import { useCallback, useEffect, useState } from 'react'

import type { CardWorktree } from '../gen/bindings'
import { Confirm } from './Confirm'
import { bytes } from './disk'
import { ask, commands } from './live'
import { useShell } from './useShell'
import { Sources } from './Sources'
import { WorktreeBase } from './WorktreeBase'

/*
 * The checkouts the cards of this project have.
 *
 * Nothing here removes anything on its own. A worktree holding uncommitted
 * work is refused by the backend, and the refusal is what fills this dialog —
 * so the number of files and lines comes from git, not from the screen's idea
 * of it.
 */

export function Worktrees(): React.JSX.Element {
  const { project } = useShell()
  const [rows, setRows] = useState<readonly CardWorktree[]>([])
  const [total, setTotal] = useState<number | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [going, setGoing] = useState<CardWorktree | null>(null)

  const reload = useCallback(() => {
    if (!project) return
    void ask(() => commands.worktreeList(project.id)).then((answer) => {
      setRows(answer.data?.worktrees ?? [])
      setTotal(answer.data?.diskBytes ?? null)
      setError(answer.error)
    })
  }, [project])

  useEffect(reload, [reload])

  const remove = (row: CardWorktree, evenDirty: boolean): void => {
    if (!project) return
    void ask(() => commands.worktreeRemove(project.id, row.cardId, evenDirty)).then((answer) => {
      setError(answer.error)
      setGoing(null)
      reload()
    })
  }

  const room = bytes(total)

  return (
    <>
      <div className="prefs__hrow">
        <h1 className="prefs__h">Worktrees</h1>
        {room && <span className="pref__d">{room} on disk</span>}
      </div>

      <WorktreeBase />
      <Sources />

      {error && <p className="pref__d">{error}</p>}
      {rows.length === 0 && !error && (
        <p className="pref__d">No card has a checkout of its own yet.</p>
      )}

      {rows.map((row) => (
        <div className="pref" key={row.cardId}>
          <span className="pref__body">
            <span className="pref__t">
              {row.cardTitle ?? row.folder}
              {row.orphan && <span className="pill"> orphan</span>}
            </span>
            <span className="pref__d">
              {[
                row.branch,
                bytes(row.diskBytes),
                row.uncommittedFiles > 0
                  ? `${row.uncommittedFiles} uncommitted file(s), ${row.uncommittedLines} line(s)`
                  : null,
              ]
                .filter(Boolean)
                .join(' · ')}
            </span>
          </span>
          <button className="btn" onClick={() => setGoing(row)}>
            Remove
          </button>
        </div>
      ))}

      {going && (
        <Confirm
          title={`Remove the checkout for “${going.cardTitle ?? going.folder}”?`}
          danger="Remove"
          body={
            <>
              The folder goes; <b>the branch stays</b>, with everything committed on it.
              {going.uncommittedFiles > 0 && (
                <>
                  {' '}
                  <b>
                    {going.uncommittedFiles} file(s) and {going.uncommittedLines} line(s) have not
                    been committed and would be lost.
                  </b>
                </>
              )}
            </>
          }
          onClose={() => setGoing(null)}
          onConfirm={() => remove(going, going.uncommittedFiles > 0)}
        />
      )}
    </>
  )
}
