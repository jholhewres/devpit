import { useEffect, useState } from 'react'

import { onFilesDragging, onFilesDropped } from './window'

/*
 * Where a drop will go, drawn over the whole pane while files hover.
 *
 * The window catches the drop, not an element, so without this nothing says
 * which chat will take the files. Only the chat in front listens.
 */

export function DropTarget({
  mine,
  onDrop,
}: {
  mine: boolean
  onDrop: (paths: readonly string[]) => void
}): React.JSX.Element | null {
  const [hovering, setHovering] = useState(false)
  /* The drop lands on the window, not on a pane, so only the chat in front
     takes it. */
  useEffect(() => {
    if (!mine) return
    return onFilesDropped(onDrop)
  }, [mine, onDrop])
  useEffect(() => {
    if (!mine) return setHovering(false)
    return onFilesDragging(setHovering)
  }, [mine])

  if (!hovering) return null
  return (
    <div className="chat__drop" aria-hidden="true">
      <span className="chat__dropt">Drop to attach to this chat</span>
    </div>
  )
}
