import { useEffect, useRef } from 'react'

import { Row } from './Row'
import { matching, ordered } from './tree'
import { onTreeKeyDown, rove } from './useTreeKeys'

import type { FileNode } from '../gen/bindings'

export function Tree({
  projectId,
  nodes,
  version,
  query,
  current,
  collapsed,
  onOpen,
}: {
  projectId: string
  nodes: readonly FileNode[]
  version: number
  query: string
  current: string | null
  collapsed: number
  onOpen: (path: string) => void
}): React.JSX.Element {
  const keep = matching(nodes, query)
  const ref = useRef<HTMLDivElement>(null)
  // One tab stop for the whole tree, not one per row — see `rove`.
  useEffect(() => (ref.current ? rove(ref.current) : undefined), [])

  return (
    <div className="tree" ref={ref} onKeyDown={onTreeKeyDown}>
      {ordered(nodes).map((node) => (
        <Row
          key={node.path}
          projectId={projectId}
          node={node}
          version={version}
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
