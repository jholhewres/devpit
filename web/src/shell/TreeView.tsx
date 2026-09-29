import { useCallback, useEffect, useRef } from 'react'

import { folderUnder, useFileDrop } from './fileDrop'
import { ask, commands } from './live'
import { changed } from './treeChanged'

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

  /* Files dropped from outside are copied in: onto a folder, into it; onto
     a file, beside it; onto the empty space, at the root. */
  const dropped = useCallback(
    (paths: readonly string[], under: Element | null) =>
      void ask(() => commands.pathImport(projectId, null, folderUnder(under), [...paths])).then(() => changed()),
    [projectId],
  )
  const hovering = useFileDrop(ref, dropped)

  return (
    <div className="tree" ref={ref} onKeyDown={onTreeKeyDown} data-drop={hovering ? 'true' : undefined}>
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
