import { useState } from 'react'

import type { Change } from '../gen/bindings'
import { ChangeRows } from './ChangeRows'
import { grouped, stageable } from './changes'
import { counted, primary } from './primary'
import { DiscardConfirm } from './DiscardConfirm'
import { ask, commands } from './live'
import { Skeleton } from './Skeleton'
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
  const { project, show, active } = useShell()
  /* The row of the file on screen is marked, so the list and the pane agree
     about where you are. A panel that looks the same whatever is open makes
     you read the tab bar to find out. */
  const onScreen = active?.kind === 'diff' ? (active.path ?? null) : null

  /* A row in Changes opens the diff, not the file: the question the panel is
     answering is what changed, and the file alone does not answer it. */
  const openDiff = (path: string): void =>
    show('diff', { id: `diff:${path}`, path, title: `${path.split('/').pop()} diff` })
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState(false)
  const [said, setSaid] = useState<string | null>(null)
  const [discarding, setDiscarding] = useState<Change | null>(null)

  const groups = grouped(tree.changes)
  const act1 = primary(tree.changes, message)
  const here = project?.worktrees.find((worktree) => worktree.current) ?? project?.worktrees[0]

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
  const discard = (paths: string[]): void =>
    act(() => ask(() => commands.changesDiscard(project!.id, null, paths)))

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
          <span>{title}</span>
          <span className="git__count">{rows.length}</span>
          {/* The section's own action, beside its name. `Unstage all` used to
              be a chip at the top that applied to a group two screens down. */}
          <button
            className="gitrow__act"
            disabled={busy}
            onClick={() =>
              staged
                ? unstage(rows.map((change) => change.path))
                : stage(rows.map((change) => change.path))
            }
            title={staged ? 'Take all of these out' : 'Put all of these in'}
            aria-label={`${staged ? 'Unstage' : 'Stage'} everything ${title.toLowerCase()}`}
          >
            {staged ? '−' : '+'}
          </button>
        </div>
        <ChangeRows
          changes={rows}
          staged={staged}
          busy={busy}
          onOpen={openDiff}
          onScreen={onScreen}
          onStage={staged ? unstage : stage}
          onDiscard={setDiscarding}
        />
      </>
    )

  return (
    <>
      <div className="git__head">
        <div className="git__row">
          <span className="git__branch">{here?.branch ?? 'Changes'}</span>
          <span className="git__up">
            {here && here.ahead > 0 && <span>&uarr;{here.ahead}</span>}
            {here && here.behind > 0 && <span>&darr;{here.behind}</span>}
            <span className="add">+{counted(tree.totals.added)}</span>
            <span className="del">&minus;{counted(tree.totals.removed)}</span>
          </span>
        </div>

        {/* At the top, and that is the change: the message used to sit under a
            list forty rows long, so the field you were composing in moved as
            you read and was off the screen by the time you had read it. */}
        <textarea
          className="git__msg"
          placeholder="What changed, and why"
          value={message}
          onChange={(event) => setMessage(event.target.value)}
          aria-label="Commit message"
        />

        {/* One button, not three. What it says is decided in `primary.ts`. */}
        <button
          className="btn btn--go git__go"
          disabled={busy || act1.disabled}
          title={act1.why ?? undefined}
          onClick={() => (act1.doing === 'stage' ? stage(stageable(tree.changes)) : commit())}
        >
          {act1.label}
        </button>
        {act1.why && <span className="git__why">{act1.why}</span>}
        {said && <span className="git__why">{said}</span>}
      </div>

      <div className="git__body">
        {tree.loading && tree.changes.length === 0 && <Skeleton />}
        {!tree.loading && tree.changes.length === 0 && (
          <div className="exempty">
            <span className="exempty__t">Nothing changed.</span>
            {said && <span className="exempty__d">{said}</span>}
          </div>
        )}
        <Group title="Staged" rows={groups.staged} staged={true} />
        <Group title="Changed" rows={groups.changed} staged={false} />
        <Group title="Untracked" rows={groups.untracked} staged={false} />
      </div>

      {discarding && (
        <DiscardConfirm
          change={discarding}
          onClose={() => setDiscarding(null)}
          onConfirm={() => {
            discard([discarding.path])
            setDiscarding(null)
          }}
        />
      )}
    </>
  )
}
