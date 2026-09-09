import { useState } from 'react'

import type { Change } from '../gen/bindings'
import { committable, grouped, stageable } from './changes'
import { ask, commands } from './live'
import { mark } from './tree'
import { useShell } from './useShell'
import type { UseTree } from './useTree'

/*
 * What has changed, and what a commit would take.
 *
 * Nothing here happens on its own: staging is a click and committing is a
 * click. The lists come from git each time rather than being predicted from
 * the click, so the panel cannot drift from the index.
 */

export function Changes({ tree }: { tree: UseTree }): React.JSX.Element {
  const { project, show } = useShell()

  /* A row in Changes opens the diff, not the file: the question the panel is
     answering is what changed, and the file alone does not answer it. */
  const openDiff = (path: string): void =>
    show('diff', { id: `diff:${path}`, path, title: `${path.split('/').pop()} diff` })
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState(false)
  const [said, setSaid] = useState<string | null>(null)

  const groups = grouped(tree.changes)

  const act = (
    call: () => Promise<unknown>,
  ): void => {
    if (!project) return
    setBusy(true)
    void call()
      .then(() => tree.reload())
      .finally(() => setBusy(false))
  }

  const stage = (paths: string[]): void =>
    act(() => ask(() => commands.changesStage(project!.id, null, paths)))
  const unstage = (paths: string[]): void =>
    act(() => ask(() => commands.changesUnstage(project!.id, null, paths)))

  const commit = (): void => {
    if (!project) return
    setBusy(true)
    void ask(() => commands.changesCommit(project.id, null, message))
      .then((answer) => {
        if (answer.data) {
          setSaid(`${answer.data.sha} · ${answer.data.subject}`)
          setMessage('')
        } else {
          setSaid(answer.error)
        }
        tree.reload()
      })
      .finally(() => setBusy(false))
  }

  const Row = ({ change, staged }: { change: Change; staged: boolean }): React.JSX.Element => (
    <div className="gitrow gitrow--file">
      <button className="gitrow__open" onClick={() => openDiff(change.path)}>
        <span className="gitrow__n">{change.path}</span>
      </button>
      <span className="gitrow__end">
        {change.added > 0 && <span className="add">+{change.added}</span>}
        {change.removed > 0 && <span className="del">&minus;{change.removed}</span>}
        <span className={`row__g row__g--${change.status}`}>{mark(change.status)}</span>
        <button
          className="gitrow__act"
          disabled={busy}
          onClick={() => (staged ? unstage([change.path]) : stage([change.path]))}
          title={staged ? 'Take out of the commit' : 'Put in the commit'}
        >
          {staged ? '−' : '+'}
        </button>
      </span>
    </div>
  )

  const Group = ({
    title,
    rows,
    staged,
  }: {
    title: string
    rows: readonly Change[]
    staged: boolean
  }): React.JSX.Element | null =>
    rows.length === 0 ? null : (
      <>
        <div className="git__group">
          {title} <span>{rows.length}</span>
        </div>
        {rows.map((change) => (
          <Row key={change.path} change={change} staged={staged} />
        ))}
      </>
    )

  return (
    <>
      <div className="git__head">
        <span className="git__branch">Changes</span>
        <span className="git__up">
          <span className="add">+{tree.totals.added}</span>
          <span className="del">&minus;{tree.totals.removed}</span>
        </span>
        <button
          className="chip"
          disabled={busy || stageable(tree.changes).length === 0}
          onClick={() => stage(stageable(tree.changes))}
        >
          Stage all
        </button>
      </div>

      <div className="git__body">
        {tree.changes.length === 0 && (
          <div className="exempty">
            <span className="exempty__t">Nothing changed.</span>
            {said && <span className="exempty__d">{said}</span>}
          </div>
        )}
        <Group title="Staged" rows={groups.staged} staged={true} />
        <Group title="Changed" rows={groups.changed} staged={false} />
        <Group title="Untracked" rows={groups.untracked} staged={false} />
      </div>

      {tree.changes.length > 0 && (
        <div className="git__commit">
          <textarea
            className="git__msg"
            placeholder="What changed, and why"
            value={message}
            onChange={(event) => setMessage(event.target.value)}
            aria-label="Commit message"
          />
          <button
            className="btn btn--go"
            disabled={busy || !committable(tree.changes, message)}
            onClick={commit}
          >
            Commit {groups.staged.length} file(s)
          </button>
          {said && <span className="exempty__d">{said}</span>}
        </div>
      )}
    </>
  )
}
