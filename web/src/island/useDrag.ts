import { useRef } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'

/*
 * Dragging the island to another screen.
 *
 * A press that moves further than a few pixels hands the window to the
 * window manager; Rust settles it at the top of whichever screen it is
 * dropped on. A press that does not move stays a click — and a click that
 * ended a drag is not one.
 */

const FAR = 6

export interface Drag {
  readonly onPointerDown: (event: React.PointerEvent) => void
  readonly onPointerMove: (event: React.PointerEvent) => void
  readonly onPointerUp: () => void
  /** Whether the click now arriving ended a drag, which then forgets it. */
  readonly wasDrag: () => boolean
}

export function useDrag(): Drag {
  const press = useRef<{ x: number; y: number } | null>(null)
  const dragged = useRef(false)
  return {
    onPointerDown: (event) => {
      dragged.current = false
      const onButton = (event.target as HTMLElement).closest('button')
      press.current = event.button === 0 && !onButton ? { x: event.screenX, y: event.screenY } : null
    },
    onPointerMove: (event) => {
      const from = press.current
      if (!from || (event.buttons & 1) === 0) return
      if (Math.hypot(event.screenX - from.x, event.screenY - from.y) < FAR) return
      press.current = null
      dragged.current = true
      void getCurrentWindow()
        .startDragging()
        .catch(() => {
          /* Not in the app, or the system refused: it stays where it is. */
        })
    },
    onPointerUp: () => {
      press.current = null
    },
    wasDrag: () => {
      const was = dragged.current
      dragged.current = false
      return was
    },
  }
}
