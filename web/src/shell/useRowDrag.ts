import { useState } from 'react'

import { ask, commands } from './live'
import { canDropOn, currentlyDragged, destinationIn, DRAG_MIME, edgeScrollDelta, startDragging, stopDragging } from './rowDrag'
import { changed } from './useTree'

export interface RowDrag {
  readonly draggable: true
  /** Whether a drag that could actually drop here is over this row right now. */
  readonly over: boolean
  readonly onDragStart: (event: React.DragEvent<HTMLButtonElement>) => void
  readonly onDragOver: (event: React.DragEvent<HTMLButtonElement>) => void
  readonly onDragLeave: () => void
  readonly onDrop: (event: React.DragEvent<HTMLButtonElement>) => void
  readonly onDragEnd: () => void
}

/* The nearest ancestor that actually scrolls — `.tree` in the right panel,
   `.scroll` in the full-width Files pane. Read off the DOM rather than a
   class name, so a future container that also scrolls needs no change here. */
function scroller(from: HTMLElement): HTMLElement | null {
  for (let node = from.parentElement; node; node = node.parentElement) {
    if (node.scrollHeight > node.clientHeight && getComputedStyle(node).overflowY !== 'visible') return node
  }
  return null
}

/* Drag-and-drop for one row: a file or folder dragged onto a folder row
   moves there, via the same `path.move` rename already uses. Every row is a
   source; only a folder is a target, so `over` and the drop itself are
   no-ops when `folder` is false. */
export function useRowDrag(projectId: string, path: string, folder: boolean): RowDrag {
  const [over, setOver] = useState(false)

  const onDragStart = (event: React.DragEvent<HTMLButtonElement>): void => {
    event.dataTransfer.setData(DRAG_MIME, path)
    event.dataTransfer.effectAllowed = 'move'
    startDragging(path)
  }

  const onDragOver = (event: React.DragEvent<HTMLButtonElement>): void => {
    if (!event.dataTransfer.types.includes(DRAG_MIME)) return
    /* The scroll nudge is unconditional: it is a plain property write, not
       the drop itself, so it has no business waiting on whether this row
       could accept one. */
    const container = scroller(event.currentTarget)
    if (container) container.scrollTop += edgeScrollDelta(container.getBoundingClientRect(), event.clientY)

    const dragged = currentlyDragged()
    const allowed = folder && dragged !== null && canDropOn(dragged, path)
    setOver(allowed)
    if (!allowed) return
    event.preventDefault()
    event.dataTransfer.dropEffect = 'move'
  }

  const onDragLeave = (): void => setOver(false)

  const onDrop = (event: React.DragEvent<HTMLButtonElement>): void => {
    event.preventDefault()
    setOver(false)
    if (!folder) return
    const dragged = event.dataTransfer.getData(DRAG_MIME)
    if (!dragged || !canDropOn(dragged, path)) return
    void ask(() => commands.pathMove(projectId, null, dragged, destinationIn(dragged, path))).then(changed)
  }

  const onDragEnd = (): void => {
    stopDragging()
    setOver(false)
  }

  return { draggable: true, over, onDragStart, onDragOver, onDragLeave, onDrop, onDragEnd }
}
