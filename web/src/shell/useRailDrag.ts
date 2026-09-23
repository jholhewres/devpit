import { useEffect, useRef, useState } from 'react'

/*
 * Moving things in the rail by dragging them, with the pointer rather than
 * HTML drag-and-drop.
 *
 * WebKitGTK's drag-and-drop never started a drag here — the rows took the
 * press, showed nothing, and dropped nothing — so every move the rail offered
 * was a move nobody could make. Pointer events are the same on every engine:
 * a press, a few pixels of travel to tell a drag from a click, and the row
 * under the pointer as the target, read from the DOM (`data-drop`).
 */

export type Grab = { kind: 'project'; id: string } | { kind: 'group'; name: string }

/** The row a drop would land on, and on which side of it. */
export interface Spot {
  readonly kind: 'project' | 'group'
  readonly key: string
  readonly after: boolean
}

/* Travel before a press becomes a drag: less and a shaky click drags. */
const SLOP = 5
/* Near the list's top or bottom edge, a drag scrolls it. */
const EDGE = 28

export interface RailDrag {
  readonly grab: Grab | null
  readonly spot: Spot | null
  press: (grab: Grab) => (event: React.PointerEvent) => void
  /** False for the click a drag ends with: it was a drop, not a click. */
  clicked: () => boolean
}

export function useRailDrag(list: React.RefObject<HTMLElement | null>, onDrop: (grab: Grab, spot: Spot) => void): RailDrag {
  const [grab, setGrab] = useState<Grab | null>(null)
  const [spot, setSpot] = useState<Spot | null>(null)
  const pressed = useRef<{ grab: Grab; x: number; y: number } | null>(null)
  const dropped = useRef(false)
  const drop = useRef(onDrop)
  drop.current = onDrop

  useEffect(() => {
    let active: Grab | null = null
    let at: Spot | null = null

    const end = (): void => {
      pressed.current = null
      active = null
      at = null
      document.body.style.removeProperty('cursor')
      setGrab(null)
      setSpot(null)
    }

    const move = (event: PointerEvent): void => {
      const press = pressed.current
      if (!press) return
      if (!active) {
        if (Math.hypot(event.clientX - press.x, event.clientY - press.y) < SLOP) return
        active = press.grab
        document.body.style.cursor = 'grabbing'
        setGrab(active)
      }
      event.preventDefault()
      const row = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('[data-drop]')
      if (row?.dataset.drop && row.dataset.key) {
        const box = row.getBoundingClientRect()
        const next: Spot = {
          kind: row.dataset.drop === 'group' ? 'group' : 'project',
          key: row.dataset.key,
          after: event.clientY > box.top + box.height / 2,
        }
        if (next.kind !== at?.kind || next.key !== at.key || next.after !== at.after) {
          at = next
          setSpot(next)
        }
      }
      const scroller = list.current
      if (scroller) {
        const box = scroller.getBoundingClientRect()
        if (event.clientY < box.top + EDGE) scroller.scrollTop -= 8
        else if (event.clientY > box.bottom - EDGE) scroller.scrollTop += 8
      }
    }

    const up = (): void => {
      if (active) {
        dropped.current = true
        if (at) drop.current(active, at)
        /* The click that follows this release is the drop's, not a click. */
        setTimeout(() => (dropped.current = false), 0)
      }
      end()
    }

    const key = (event: KeyboardEvent): void => {
      if (event.key === 'Escape' && active) end()
    }

    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up)
    window.addEventListener('pointercancel', end)
    window.addEventListener('keydown', key)
    return () => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', up)
      window.removeEventListener('pointercancel', end)
      window.removeEventListener('keydown', key)
    }
  }, [list])

  return {
    grab,
    spot,
    press: (one) => (event) => {
      if (event.button !== 0) return
      pressed.current = { grab: one, x: event.clientX, y: event.clientY }
    },
    clicked: () => !dropped.current,
  }
}
