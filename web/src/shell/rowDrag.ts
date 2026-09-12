/*
 * The pure rules a row drag has to satisfy, apart from the DOM wiring in
 * `useRowDrag` so each can be tested without faking a drag event.
 */

/** What `dataTransfer` carries: the path being dragged, nothing else. */
export const DRAG_MIME = 'application/x-devpit-path'

/** Where `dragged` lands once dropped on `targetFolder`. */
export function destinationIn(dragged: string, targetFolder: string): string {
  const name = dragged.split('/').pop() ?? dragged
  return targetFolder ? `${targetFolder}/${name}` : name
}

/* A folder can't become its own child. Checked against the *path*, not a
   walk of the in-memory tree — a plain string is what survives the trip
   through `dataTransfer`, and it is enough: a descendant's path always
   starts with its ancestor's path plus a slash, and nothing else does.

   The other refusal is quieter: dropping something back where it already
   is. The paths differ so the checks above let it through, but the
   destination it would produce is the source itself — a no-op the backend
   would answer with `AlreadyExists` rather than simply doing nothing. */
export function canDropOn(dragged: string, targetFolder: string): boolean {
  if (dragged === targetFolder) return false
  if (targetFolder.startsWith(`${dragged}/`)) return false
  return destinationIn(dragged, targetFolder) !== dragged
}

/* `dataTransfer.getData` is deliberately unreadable during `dragover` — the
   spec only allows it on `drop` and `dragend` — but `dragover` is exactly
   where the highlight needs to know whether *this* drag could land here.
   The drag never leaves this document, so a module-level value answers it:
   set on dragstart, cleared on dragend. */
let dragging: string | null = null

export function startDragging(path: string): void {
  dragging = path
}

export function stopDragging(): void {
  dragging = null
}

export function currentlyDragged(): string | null {
  return dragging
}

const EDGE = 40 // px of the container's own edge that starts a scroll
const STEP = 16 // px nudged per dragover tick inside that band

/* How far to nudge a scroll container's `scrollTop` for a pointer at `y`
   within `rect` — zero outside the edge bands, a constant step inside one,
   signed toward that edge. Pulled out so a test can hand it a plain
   rectangle instead of faking a real scroll container mid-drag. */
export function edgeScrollDelta(rect: { top: number; bottom: number }, y: number): number {
  if (y < rect.top + EDGE) return -STEP
  if (y > rect.bottom - EDGE) return STEP
  return 0
}
