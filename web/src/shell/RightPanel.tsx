import { useCallback, useState } from 'react'
import { commands } from '../gen/bindings'
import type { Change, FileNode, Project, Worktree } from '../gen/bindings'
import { useLoad } from '../project/load'
import { ChevronGlyph } from './glyphs'

/**
 * What the code is, for the open project.
 *
 * Three tabs and no overlap with the left column: the sidebar answers what is
 * happening, this answers what exists. Anything that appeared on both sides
 * would have to be kept in agreement forever.
 */

type Tab = 'files' | 'changes' | 'worktrees'

/**
 * One level of the tree, fetched when it is opened.
 *
 * The whole tree is never asked for: a monorepo has hundreds of thousands of
 * files and the screen draws only what is expanded. Ordering is settled on the
 * Rust side, so this renders the order it was given rather than sorting a
 * second time against different rules.
 */
/**
 * The children of one open directory.
 *
 * A component of its own, mounted only while the row is open, so a closed
 * folder cannot fetch anything. Guarding this with a flag inside the row would
 * work until someone forgot it, and the cost of forgetting is one `git status`
 * over the whole repository per collapsed folder on screen.
 */
function TreeChildren({
  path,
  depth,
  projectId,
  worktreeId,
  onOpenFile
}: {
  path: string
  depth: number
  projectId: string
  worktreeId: string | null
  onOpenFile: (path: string) => void
}): React.JSX.Element | null {
  const load = useCallback(
    () => commands.projectTree(projectId, worktreeId, path),
    [projectId, worktreeId, path]
  )
  const { state } = useLoad(load, [projectId, worktreeId, path])

  if (state.status === 'loading') return null
  if (state.status === 'failed') {
    return (
      <p className="empty" style={{ paddingLeft: 25 + depth * 13 }}>
        {state.message}
      </p>
    )
  }

  return (
    <>
      {state.data.nodes.map((child) => (
        <TreeRow
          key={child.path}
          node={child}
          depth={depth}
          projectId={projectId}
          worktreeId={worktreeId}
          onOpenFile={onOpenFile}
        />
      ))}
    </>
  )
}

function TreeRow({
  node,
  depth,
  projectId,
  worktreeId,
  onOpenFile
}: {
  node: FileNode
  depth: number
  projectId: string
  worktreeId: string | null
  onOpenFile: (path: string) => void
}): React.JSX.Element {
  const isDir = node.children !== null
  const [open, setOpen] = useState(false)

  return (
    <>
      <button
        type="button"
        className="tree-row"
        data-kind={isDir ? 'dir' : 'file'}
        data-status={node.status}
        data-open={isDir ? open : undefined}
        style={{ paddingLeft: 12 + depth * 13 }}
        title={node.path}
        onClick={() => (isDir ? setOpen((was) => !was) : onOpenFile(node.path))}
      >
        <span className="tree-row__twist">{isDir ? <ChevronGlyph /> : null}</span>
        <span className="tree-row__name">{node.name}</span>
      </button>

      {isDir && open ? (
        <TreeChildren
          path={node.path}
          depth={depth + 1}
          projectId={projectId}
          worktreeId={worktreeId}
          onOpenFile={onOpenFile}
        />
      ) : null}
    </>
  )
}

const MARK: Record<Change['status'], string> = {
  clean: '·',
  modified: 'M',
  added: 'A',
  deleted: 'D',
  untracked: '?'
}

/**
 * The bar is the size of the edit, not its sign.
 *
 * Two numbers side by side get read one at a time; a bar answers "how big is
 * this change" before you have read anything, which is the question you have
 * while scanning twenty rows. Widths are shares of the largest row, so a
 * one-line fix stays small next to a rewrite.
 */
function ChangeRow({ change, of }: { change: Change; of: number }): React.JSX.Element {
  // The separator travels with the filename, never with the directory.
  // Clipping the directory from the front needs `direction: rtl`, and bidi
  // moves a trailing slash to the other end — `/web/src/shellAppShell.tsx`.
  // A directory that begins and ends with a letter has nothing to move.
  const cut = change.path.lastIndexOf('/')
  const dir = cut === -1 ? '' : change.path.slice(0, cut)
  const file = cut === -1 ? change.path : change.path.slice(cut)
  const touched = change.added + change.removed
  const share = of === 0 ? 0 : touched / of

  return (
    <button type="button" className="change" data-status={change.status} title={change.path}>
      <span className="change__mark">{MARK[change.status]}</span>
      <span className="change__path">
        {dir ? <span className="change__dir">{dir}</span> : null}
        <span className="change__file">{file}</span>
      </span>
      <span className="change__bar" aria-hidden="true">
        <i style={{ flexGrow: change.added * share }} />
        <b style={{ flexGrow: change.removed * share }} />
      </span>
      {/* A zero side is left out. `+118 −0` makes the eye check a number that
          was never in question. */}
      <span className="change__delta">
        {change.added > 0 ? <i>+{change.added}</i> : null}
        {change.removed > 0 ? <b>−{change.removed}</b> : null}
      </span>
    </button>
  )
}

/** `4↑ 6↓` in one glance, and a word when there is no drift to report. */
function Drift({ worktree }: { worktree: Worktree }): React.JSX.Element {
  if (worktree.ahead === 0 && worktree.behind === 0) {
    return (
      <span className="drift" data-clean="true">
        in sync
      </span>
    )
  }
  return (
    <span className="drift">
      {worktree.ahead > 0 ? <i>{worktree.ahead}↑</i> : null}
      {worktree.behind > 0 ? <b>{worktree.behind}↓</b> : null}
    </span>
  )
}

function WorktreeRow({
  worktree,
  active,
  onSelect
}: {
  worktree: Worktree
  active: boolean
  onSelect: () => void
}): React.JSX.Element {
  return (
    <button
      type="button"
      className="worktree"
      data-current={active}
      title={`~/${worktree.folder}`}
      onClick={onSelect}
    >
      <span className="worktree__line">
        <span className="worktree__branch">{worktree.branch}</span>
        <Drift worktree={worktree} />
      </span>
      <span className="worktree__meta">
        <span className="worktree__folder">~/{worktree.folder}</span>
        {/* Unread is not zero, here as everywhere. */}
        <span>
          {worktree.dirtyFiles === null
            ? 'unread'
            : worktree.dirtyFiles === 0
              ? 'clean'
              : `${worktree.dirtyFiles} dirty`}
        </span>
      </span>
    </button>
  )
}

export function RightPanel({
  project,
  worktreeId,
  onSelectWorktree,
  onStartResize,
  onOpenFile
}: {
  project: Project
  worktreeId: string | null
  onSelectWorktree: (id: string) => void
  onStartResize: (event: React.PointerEvent) => void
  onOpenFile: (path: string) => void
}): React.JSX.Element {
  const [tab, setTab] = useState<Tab>('files')

  const loadTree = useCallback(
    () => commands.projectTree(project.id, worktreeId, ''),
    [project.id, worktreeId]
  )
  const tree = useLoad(loadTree, [project.id, worktreeId])

  const loadChanges = useCallback(
    () => commands.projectChanges(project.id, worktreeId),
    [project.id, worktreeId]
  )
  const changes = useLoad(loadChanges, [project.id, worktreeId])

  const largest =
    changes.state.status === 'ready'
      ? changes.state.data.changes.reduce(
          (most, change) => Math.max(most, change.added + change.removed),
          0
        )
      : 0

  return (
    <section className="panel" aria-label="Files, changes and worktrees">
      <div
        className="divider"
        data-edge="panel"
        role="separator"
        aria-orientation="vertical"
        onPointerDown={onStartResize}
      />

      <div className="panel__tabs">
        <button
          type="button"
          className="panel__tab"
          data-active={tab === 'files'}
          onClick={() => setTab('files')}
        >
          Files
        </button>
        <button
          type="button"
          className="panel__tab"
          data-active={tab === 'changes'}
          onClick={() => setTab('changes')}
        >
          Changes
          {changes.state.status === 'ready' && changes.state.data.changes.length > 0 ? (
            <span className="panel__count">{changes.state.data.changes.length}</span>
          ) : null}
        </button>
        <button
          type="button"
          className="panel__tab"
          data-active={tab === 'worktrees'}
          onClick={() => setTab('worktrees')}
        >
          Worktrees
          {project.worktrees.length > 1 ? (
            <span className="panel__count">{project.worktrees.length}</span>
          ) : null}
        </button>
      </div>

      {tab === 'changes' && changes.state.status === 'ready' && changes.state.data.changes.length > 0 ? (
        <div className="panel__summary">
          <span>{changes.state.data.changes.length} files</span>
          <span className="panel__summary-spacer" />
          {/* The totals come from the command, not from adding up a list this
              side may only have part of. */}
          <span className="drift">
            <i>+{changes.state.data.added}</i>
            <b>−{changes.state.data.removed}</b>
          </span>
        </div>
      ) : null}

      <div className="panel__body scroll">
        {tab === 'files' ? (
          tree.state.status === 'loading' ? (
            <p className="empty">Reading…</p>
          ) : tree.state.status === 'failed' ? (
            <p className="empty">{tree.state.message}</p>
          ) : tree.state.data.nodes.length === 0 ? (
            <p className="empty">This folder is empty.</p>
          ) : (
            tree.state.data.nodes.map((node) => (
              <TreeRow
                key={node.path}
                node={node}
                depth={0}
                projectId={project.id}
                worktreeId={worktreeId}
                onOpenFile={onOpenFile}
              />
            ))
          )
        ) : null}

        {tab === 'changes' ? (
          changes.state.status === 'loading' ? (
            <p className="empty">Reading…</p>
          ) : changes.state.status === 'failed' ? (
            <p className="empty">{changes.state.message}</p>
          ) : changes.state.data.changes.length === 0 ? (
            <p className="empty">Nothing has changed since the last commit.</p>
          ) : (
            changes.state.data.changes.map((change) => (
              <ChangeRow key={change.path} change={change} of={largest} />
            ))
          )
        ) : null}

        {tab === 'worktrees' ? (
          project.worktrees.length === 0 ? (
            <p className="empty">{project.unreadable ?? 'No checkouts.'}</p>
          ) : (
            project.worktrees.map((worktree) => (
              <WorktreeRow
                key={worktree.id}
                worktree={worktree}
                active={worktree.id === (worktreeId ?? project.worktrees[0]?.id)}
                onSelect={() => onSelectWorktree(worktree.id)}
              />
            ))
          )
        ) : null}
      </div>
    </section>
  )
}
