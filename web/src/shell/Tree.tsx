import { useState } from 'react'

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
  nodes,
  query,
  current,
  collapsed,
  onOpen,
}: {
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
  node,
  depth,
  keep,
  filtering,
  current,
  collapsed,
  onOpen,
}: {
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
  const [open, setOpen] = useState(depth === 0)
  const [shutAt, setShutAt] = useState(collapsed)
  const folded = collapsed !== shutAt ? false : open

  if (filtering && !keep.has(node.path)) return null

  const folder = node.children !== null
  const letter = mark(node.status)
  const showing = filtering ? true : folded

  return (
    <>
      <button
        className="row"
        data-ctx="file"
        aria-current={node.path === current}
        style={{ paddingLeft: 8 + depth * 13 }}
        onClick={() => {
          if (!folder) return onOpen(node.path)
          setShutAt(collapsed)
          setOpen(!folded)
        }}
      >
        <span className="row__chev">{folder && <Chev open={showing} />}</span>
        <span className="row__ico">{folder ? <Dir /> : <Doc />}</span>
        <span className="row__n">{node.name}</span>
        {letter && <span className={`row__g row__g--${node.status}`}>{letter}</span>}
      </button>
      {folder &&
        showing &&
        ordered(node.children ?? []).map((child) => (
          <Row
            key={child.path}
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
