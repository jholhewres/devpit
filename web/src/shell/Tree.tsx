import { useState } from 'react'

import { ask, commands } from './live'

import type { FileNode } from '../gen/bindings'
import { mark, matching, ordered } from './tree'

const Chev = ({ open }: { open: boolean }): React.JSX.Element => (
  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ transform: open ? 'rotate(90deg)' : 'none' }}>
    <path d="m9 6 6 6-6 6" />
  </svg>
)

const Doc = (): React.JSX.Element => (
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
    <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
    <path d="M14 2v5h5" />
  </svg>
)

const Dir = (): React.JSX.Element => (
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
    <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
  </svg>
)

export function Tree({
  projectId,
  nodes,
  query,
  current,
  collapsed,
  onOpen,
}: {
  projectId: string
  nodes: readonly FileNode[]
  query: string
  current: string | null
  collapsed: number
  onOpen: (path: string) => void
}): React.JSX.Element {
  const keep = matching(nodes, query)
  return (
    <div className="tree">
      {ordered(nodes).map((node) => (
        <Row
          key={node.path}
          projectId={projectId}
          node={node}
          depth={0}
          keep={keep}
          filtering={query.trim().length > 0}
          current={current}
          collapsed={collapsed}
          onOpen={onOpen}
        />
      ))}
    </div>
  )
}

function Row({
  projectId,
  node,
  depth,
  keep,
  filtering,
  current,
  collapsed,
  onOpen,
}: {
  projectId: string
  node: FileNode
  depth: number
  keep: Set<string>
  filtering: boolean
  current: string | null
  collapsed: number
  onOpen: (path: string) => void
}): React.JSX.Element | null {
  /* `collapsed` is a counter, not a flag: Collapse all bumps it and every row
     re-reads it, which shuts folders the reader opened by hand. */
  const [open, setOpen] = useState(false)
  const [shutAt, setShutAt] = useState(collapsed)
  /* The tree arrives a level at a time, so a folder fetches its own children
     the first time it is opened. */
  const [children, setChildren] = useState<readonly FileNode[] | null>(
    node.children?.length ? node.children : null,
  )
  const folded = collapsed !== shutAt ? false : open

  if (filtering && !keep.has(node.path)) return null

  const folder = node.children !== null
  const letter = mark(node.status)
  const showing = filtering ? true : folded

  function toggle(): void {
    if (!folder) return onOpen(node.path)
    setShutAt(collapsed)
    const next = !folded
    setOpen(next)
    if (next && children === null) {
      void ask(() => commands.projectTree(projectId, null, node.path)).then((asked) => {
        if (asked.data) setChildren(asked.data.nodes)
      })
    }
  }

  return (
    <>
      <button
        className="row"
        data-ctx="file"
        aria-current={node.path === current}
        style={{ paddingLeft: 8 + depth * 13 }}
        onClick={toggle}
      >
        <span className="row__chev">{folder && <Chev open={showing} />}</span>
        <span className="row__ico">{folder ? <Dir /> : <Doc />}</span>
        <span className="row__n">{node.name}</span>
        {letter && <span className={`row__g row__g--${node.status}`}>{letter}</span>}
      </button>
      {folder &&
        showing &&
        ordered(children ?? []).map((child) => (
          <Row
            key={child.path}
            projectId={projectId}
            node={child}
            depth={depth + 1}
            keep={keep}
            filtering={filtering}
            current={current}
            collapsed={collapsed}
            onOpen={onOpen}
          />
        ))}
    </>
  )
}
