import { useState } from 'react'

import type { Change } from '../gen/bindings'
import { ChangeRows, type View } from './ChangeRows'
import { ChangeSection } from './ChangeSection'
import { grouped, rememberView, savedView, stageable } from './changes'
import { counted, primary } from './primary'
import { DiscardConfirm } from './DiscardConfirm'
import { List, Minus, Plus, Search, Trash, Tree, Undo } from './GitIcons'
import { ask, commands } from './live'
import { Skeleton } from './Skeleton'
import { abandoned, committed } from './typing'
import { useShell } from './useShell'
import type { UseTree } from './useTree'

/*
 * What has changed, and what a commit would take.
 *
 * Nothing here happens on its own: staging is a click and committing is a
 * click. The lists come from git each time rather than being predicted from
 * the click, so the panel cannot drift from the index.
 */

type Group = 'staged' | 'changed' | 'untracked'

export function Changes({ tree }: { tree: UseTree }): React.JSX.Element {
  const { project, show, active } = useShell()
  /* The row of the file on screen is marked, so the list and the pane agree
     about where you are. */
  const onScreen = active?.kind === 'diff' ? (active.path ?? null) : null
  /* A row in Changes opens the diff, not the file: the question the panel is
     answering is what changed, and the file alone does not answer it. */
  const openDiff = (path: string): void =>
    show('diff', { id: `diff:${path}`, path, title: `${path.split('/').pop()} diff` })

  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState(false)
  const [said, setSaid] = useState<string | null>(null)
  const [discarding, setDiscarding] = useState<readonly Change[] | null>(null)
  const [view, setView] = useState<View>(savedView)
  const [finding, setFinding] = useState<string | null>(null)
  const [shut, setShut] = useState<ReadonlySet<Group>>(() => new Set())

  const wanted = finding?.trim().toLowerCase() ?? ''
  const shown = wanted ? tree.changes.filter((change) => change.path.toLowerCase().includes(wanted)) : tree.changes
  const groups = grouped(shown)
  const act1 = primary(tree.changes, message)
  const here = project?.worktrees.find((worktree) => worktree.current) ?? project?.worktrees[0]

  const act = (call: () => Promise<unknown>): void => {
    if (!project) return
    setBusy(true)
    void call()
      .then(() => tree.reload())
      .finally(() => setBusy(false))
  }
  const stage = (paths: string[]): void => act(() => ask(() => commands.changesStage(project!.id, null, paths)))
  const unstage = (paths: string[]): void => act(() => ask(() => commands.changesUnstage(project!.id, null, paths)))
  const discard = (paths: string[]): void => act(() => ask(() => commands.changesDiscard(project!.id, null, paths)))

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
  const go = (): void => (act1.doing === 'stage' ? stage(stageable(tree.changes)) : commit())

  const turn = (next: View): void => {
    setView(next)
    rememberView(next)
  }

  const section = (group: Group, title: string, rows: readonly Change[]): React.JSX.Element => {
    const staged = group === 'staged'
    const paths = rows.map((change) => change.path)
    const name = title.toLowerCase()
    return (
      <ChangeSection
        title={title}
        count={rows.length}
        open={!shut.has(group)}
        onToggle={() =>
          setShut((was) => {
            const next = new Set(was)
            if (next.has(group)) next.delete(group)
            else next.add(group)
            return next
          })
        }
        actions={
          <>
            {!staged && (
              <button className="gitrow__act" disabled={busy} onClick={() => setDiscarding(rows)} title={group === 'untracked' ? 'Delete all' : 'Discard all'} aria-label={`Discard everything ${name}`}>
                {group === 'untracked' ? <Trash /> : <Undo />}
              </button>
            )}
            <button className="gitrow__act" disabled={busy} onClick={() => (staged ? unstage(paths) : stage(paths))} title={staged ? 'Unstage all' : 'Stage all'} aria-label={`${staged ? 'Unstage' : 'Stage'} everything ${name}`}>
              {staged ? <Minus /> : <Plus />}
            </button>
          </>
        }
      >
        <ChangeRows changes={rows} view={view} staged={staged} busy={busy} onOpen={openDiff} onScreen={onScreen} onStage={staged ? unstage : stage} onDiscard={(change) => setDiscarding([change])} />
      </ChangeSection>
    )
  }

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
          <button className="dbtn" aria-pressed={finding !== null} onClick={() => setFinding((was) => (was === null ? '' : null))} title="Filter files" aria-label="Filter files">
            <Search />
          </button>
          <button className="dbtn" onClick={() => turn(view === 'tree' ? 'list' : 'tree')} title={view === 'tree' ? 'View as list' : 'View as tree'} aria-label={view === 'tree' ? 'View as list' : 'View as tree'}>
            {view === 'tree' ? <List /> : <Tree />}
          </button>
        </div>
        {finding !== null && (
          <input className="git__find" autoFocus placeholder="Filter files" value={finding} onChange={(event) => setFinding(event.target.value)} onKeyDown={(event) => abandoned(event) && setFinding(null)} aria-label="Filter files by path" />
        )}

        {/* At the top: the message used to sit under a list forty rows long,
            and was off the screen by the time you had read what changed. */}
        <textarea
          className="git__msg"
          placeholder="Message (⌘/Ctrl+Enter to commit)"
          value={message}
          onChange={(event) => setMessage(event.target.value)}
          onKeyDown={(event) => {
            if (committed(event) && (event.metaKey || event.ctrlKey) && !busy && !act1.disabled) {
              event.preventDefault()
              go()
            }
          }}
          aria-label="Commit message"
        />
        {/* One button, not three. What it says is decided in `primary.ts`. */}
        <button className="btn btn--go git__go" disabled={busy || act1.disabled} title={act1.why ?? undefined} onClick={go}>
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
        {wanted && shown.length === 0 && <div className="exempty__t">No changed file matches.</div>}
        {section('staged', 'Staged', groups.staged)}
        {section('changed', 'Changed', groups.changed)}
        {section('untracked', 'Untracked', groups.untracked)}
      </div>

      {discarding && (
        <DiscardConfirm
          changes={discarding}
          onClose={() => setDiscarding(null)}
          onConfirm={() => {
            discard(discarding.map((change) => change.path))
            setDiscarding(null)
          }}
        />
      )}
    </>
  )
}
