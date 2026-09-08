import type { LayoutNode } from '../gen/bindings'
import { TerminalPane } from './TerminalPane'

export function LayoutView({
  projectId,
  node,
  focusedId,
  onFocus
}: {
  projectId: string
  node: LayoutNode
  focusedId: string
  onFocus: (id: string) => void
}): React.JSX.Element {
  if (node.type === 'leaf') {
    return (
      <TerminalPane
        projectId={projectId}
        paneId={node.id}
        focused={node.id === focusedId}
        onFocus={() => onFocus(node.id)}
      />
    )
  }

  const firstShare = Math.round((node.ratio ?? 0.5) * 100)
  const secondShare = 100 - firstShare

  return (
    <div
      className="pty-split"
      data-direction={node.direction}
      data-testid={`pty-split-${node.direction}`}
    >
      <div className="pty-split__side" style={{ flexGrow: firstShare, flexBasis: 0 }}>
        <LayoutView
          projectId={projectId}
          node={node.first}
          focusedId={focusedId}
          onFocus={onFocus}
        />
      </div>
      <div className="pty-split__rule" data-direction={node.direction} />
      <div className="pty-split__side" style={{ flexGrow: secondShare, flexBasis: 0 }}>
        <LayoutView
          projectId={projectId}
          node={node.second}
          focusedId={focusedId}
          onFocus={onFocus}
        />
      </div>
    </div>
  )
}
