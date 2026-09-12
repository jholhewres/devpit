import type { KeyboardEvent } from 'react'

export type RowKind = 'open-folder' | 'closed-folder' | 'file'
export type ArrowAction = 'open' | 'close' | 'first-child' | 'parent' | 'none'

/*
 * The rule everyone gets wrong: Left and Right do not just toggle a folder.
 * An open folder steps *into* itself on Right and *closes* on Left. A closed
 * folder has nothing to step into, so Right opens it instead. A file has no
 * open state at all, so Left sends it to its parent the same way a closed
 * folder's Left does. Every branch is spelled out for all three kinds, in
 * both directions, so the three cases cannot quietly collapse into two.
 */
export function arrowAction(direction: 'left' | 'right', kind: RowKind): ArrowAction {
  if (direction === 'right') {
    if (kind === 'open-folder') return 'first-child'
    if (kind === 'closed-folder') return 'open'
    return 'none'
  }
  if (kind === 'open-folder') return 'close'
  if (kind === 'closed-folder') return 'parent'
  return 'parent'
}

const MOVE_KEYS = new Set(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End'])

/* Rows are plain sibling `.row` buttons in DOM order — the same order a
   reader sees on screen, folder children rendered right after their parent —
   so the visible list for Up/Down/Home/End needs no tree walk of its own,
   just a query scoped to this tree. Left/Right reads a row's kind straight
   off its own markup (`data-kind`, `aria-expanded`) rather than reaching
   back into React state that this handler has no access to. */
export function onTreeKeyDown(event: KeyboardEvent<HTMLDivElement>): void {
  if (!MOVE_KEYS.has(event.key)) return
  const target = (event.target as HTMLElement).closest<HTMLButtonElement>('.row')
  if (!target) return
  const rows = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('.row'))
  const at = rows.indexOf(target)
  if (at < 0) return

  event.preventDefault()

  if (event.key === 'ArrowDown') return rows[at + 1]?.focus()
  if (event.key === 'ArrowUp') return rows[at - 1]?.focus()
  if (event.key === 'Home') return rows[0]?.focus()
  if (event.key === 'End') return rows[rows.length - 1]?.focus()

  const direction = event.key === 'ArrowRight' ? 'right' : 'left'
  const action = arrowAction(direction, kindOf(target))
  /* 'open' and 'close' reuse the row's own click handler rather than a
     second copy of the toggle logic — the same control a mouse click
     reaches, reached by keyboard instead. */
  if (action === 'open' || action === 'close') target.click()
  if (action === 'first-child') firstChildOf(rows, at)?.focus()
  if (action === 'parent') parentOf(rows, at)?.focus()
}

function kindOf(row: HTMLButtonElement): RowKind {
  if (row.dataset.kind !== 'folder') return 'file'
  return row.getAttribute('aria-expanded') === 'true' ? 'open-folder' : 'closed-folder'
}

/* An open folder's own next row in the flat list — but only if that row is
   actually a level deeper. An empty open folder (children: Some([]), a real
   case the contract calls out on purpose) has no such row, so the next one
   is a sibling — Right must do nothing there, not walk sideways. */
function firstChildOf(rows: readonly HTMLButtonElement[], at: number): HTMLButtonElement | undefined {
  const next = rows[at + 1]
  if (!next) return undefined
  return Number(next.dataset.depth) > Number(rows[at].dataset.depth) ? next : undefined
}

/* The nearest earlier row one level shallower — folders always precede their
   own children in this flat list, so walking backwards for the first drop in
   `data-depth` always lands on the true parent, never a cousin. */
function parentOf(rows: readonly HTMLButtonElement[], at: number): HTMLButtonElement | undefined {
  const depth = Number(rows[at].dataset.depth)
  for (let i = at - 1; i >= 0; i--) {
    if (Number(rows[i].dataset.depth) === depth - 1) return rows[i]
  }
  return undefined
}

/*
 * Roving tabindex: a tree with five hundred visible rows is one stop in the
 * page's tab order, not five hundred. Every `.row` renders `tabIndex={-1}`;
 * this is what hands exactly one of them `0` — the last one focused, or the
 * first one whenever none currently is, which is also how the tree recovers
 * a tab stop after the one it had disappears (a folder that collapses, a
 * filter that drops a row, `.click()` swapping a row for its "show more"
 * sibling). A `MutationObserver` on the container catches all three the same
 * way, without this needing to know which kind of change just happened.
 *
 * Call once, from an effect, on the `.tree` element itself; call the
 * teardown it returns on cleanup.
 */
export function rove(container: HTMLElement): () => void {
  const activate = (row: HTMLButtonElement | null): void => {
    container.querySelectorAll<HTMLButtonElement>('.row').forEach((candidate) => {
      candidate.tabIndex = candidate === row ? 0 : -1
    })
  }

  const onFocusIn = (event: FocusEvent): void => {
    const row = (event.target as HTMLElement).closest<HTMLButtonElement>('.row')
    if (row) activate(row)
  }

  const ensureATabStop = (): void => {
    if (container.querySelector('.row[tabindex="0"]')) return
    activate(container.querySelector<HTMLButtonElement>('.row'))
  }

  container.addEventListener('focusin', onFocusIn)
  const observer = new MutationObserver(ensureATabStop)
  observer.observe(container, { childList: true, subtree: true })
  ensureATabStop()

  return () => {
    container.removeEventListener('focusin', onFocusIn)
    observer.disconnect()
  }
}
