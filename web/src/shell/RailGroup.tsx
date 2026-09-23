import { useState } from 'react'

import { ChevronDown } from './GitIcons'
import { abandoned, committed } from './typing'

/*
 * A group's heading in the rail: click to fold it, right-click to rename it
 * or ungroup its projects, drag it to put the group somewhere else, drop a
 * project on it to move the project in (`useRailDrag`). Renaming happens in place — the
 * heading becomes the field — because a dialog for one word is a detour.
 */

export function RailGroup({
  name,
  folded,
  renaming,
  over,
  grabbed,
  onToggle,
  onMenu,
  onRename,
  onPress,
  clicked,
}: {
  name: string
  folded: boolean
  renaming: boolean
  /** Where a drag over it would land: before, after, or into the group. */
  over: string | undefined
  /** It is the thing being dragged. */
  grabbed: boolean
  onToggle: () => void
  onMenu: (at: { x: number; y: number }) => void
  /** The new name, or null when the rename was abandoned. */
  onRename: (to: string | null) => void
  onPress: (event: React.PointerEvent) => void
  /** False for the click that ends a drag. */
  clicked: () => boolean
}): React.JSX.Element {
  const [typed, setTyped] = useState(name)
  if (renaming) {
    return (
      <div className="rail__group">
        <input
          className="rail__rename"
          autoFocus
          value={typed}
          aria-label={`Rename the group ${name}`}
          onChange={(event) => setTyped(event.target.value)}
          onBlur={() => onRename(typed)}
          onKeyDown={(event) => {
            if (committed(event)) onRename(typed)
            if (abandoned(event)) onRename(null)
          }}
        />
      </div>
    )
  }
  return (
    <button
      className="rail__group"
      aria-expanded={!folded}
      data-over={over}
      data-dragging={grabbed ? 'true' : undefined}
      data-drop="group"
      data-key={name}
      onPointerDown={onPress}
      onClick={() => clicked() && onToggle()}
      onContextMenu={(event) => {
        event.preventDefault()
        event.stopPropagation()
        onMenu({ x: event.clientX, y: event.clientY })
      }}
    >
      <span className="rail__gname">{name}</span>
      <span className="rail__gchev" data-open={!folded}>
        <ChevronDown size={12} />
      </span>
    </button>
  )
}
