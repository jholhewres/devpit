import { useState } from 'react'

import { ChevronDown } from './GitIcons'
import { abandoned, committed } from './typing'

/*
 * A group's heading in the rail: click to fold it, right-click to rename it
 * or ungroup its projects, drag it to put the group somewhere else, drop a
 * project on it to move the project in. Renaming happens in place — the
 * heading becomes the field — because a dialog for one word is a detour.
 */

export function RailGroup({
  name,
  folded,
  renaming,
  over,
  onToggle,
  onMenu,
  onRename,
  drag,
}: {
  name: string
  folded: boolean
  renaming: boolean
  /** Something is being dragged over it that it would take. */
  over: boolean
  onToggle: () => void
  onMenu: (at: { x: number; y: number }) => void
  /** The new name, or null when the rename was abandoned. */
  onRename: (to: string | null) => void
  drag: Pick<React.HTMLAttributes<HTMLElement>, 'onDragStart' | 'onDragEnd' | 'onDragOver' | 'onDragLeave' | 'onDrop'>
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
      data-over={over ? 'true' : undefined}
      draggable
      {...drag}
      onClick={onToggle}
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
