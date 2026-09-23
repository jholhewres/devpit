import { moved } from './wsbrowse'

/*
 * The rules every menu in the window keeps, apart from the markup so a test
 * can call them.
 *
 * One menu at a time. Right-clicks on the rail, a card, a lane or a terminal
 * stop their event so the window's own menu stays shut, and that same stop
 * left whatever menu was already open standing beside the new one.
 */

let open: (() => void) | null = null

/** Opens a menu: whatever menu was open closes. Answers the call that gives
 *  the place up again, which does nothing once another menu has taken it. */
export function claimMenu(close: () => void): () => void {
  const was = open
  open = close
  if (was && was !== close) was()
  return () => {
    if (open === close) open = null
  }
}

/** Where a menu of `size` opened at `at` goes: inside the window, 8px clear of
 *  every edge, and at the top left when it is bigger than the window. */
export function clamped(
  at: { readonly x: number; readonly y: number },
  size: { readonly width: number; readonly height: number },
  view: { readonly width: number; readonly height: number },
): { left: number; top: number } {
  return {
    left: Math.max(8, Math.min(at.x, view.width - size.width - 8)),
    top: Math.max(8, Math.min(at.y, view.height - size.height - 8)),
  }
}

/** Where a right-click asks for its menu. The Menu key and Shift+F10 send
 *  one at (0,0), so a keyboard's opens under the thing it was pressed on. */
export function menuPoint(event: {
  readonly clientX: number
  readonly clientY: number
  readonly target: EventTarget | null
}): { x: number; y: number } {
  if (event.clientX !== 0 || event.clientY !== 0 || !(event.target instanceof Element)) {
    return { x: event.clientX, y: event.clientY }
  }
  const box = event.target.getBoundingClientRect()
  return { x: box.left, y: box.bottom }
}

/** Which entry an arrow key, Home or End lands on from entry `at`, or null for
 *  a key the menu leaves alone. */
export function menuFocus(key: string, at: number, count: number): number | null {
  if (count === 0) return null
  if (key === 'Home') return 0
  if (key === 'End') return count - 1
  if (key === 'ArrowDown' || key === 'ArrowUp') return moved(at, key === 'ArrowDown' ? 1 : -1, count)
  return null
}
