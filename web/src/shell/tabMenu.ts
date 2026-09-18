import type { Entry } from './fileMenu'
import type { Sweep } from './strip'

/*
 * A tab's context menu, as data.
 *
 * Apart from the component for the same reason `fileMenu` is: a test can ask
 * whether every entry does something, which the `a_control_either_works_or_
 * goes` guard cannot — it checks that a handler exists, not that the handler
 * has anything to call.
 *
 * What Orca offers here and this does not: Pin, and Move to Split. Neither is
 * a thing this window has. A split in devpit is inside a terminal tab and
 * belongs to tmux, not to the strip, and nothing pins. An entry that cannot
 * be wired leaves the screen — AGENTS.md is explicit — so they are absent
 * rather than greyed out.
 */

export interface TabEntry extends Entry {
  readonly run?: (id: string) => void
}

export interface TabHands {
  close: (id: string) => void
  sweep: (id: string, what: Sweep) => void
}

export const tabMenu = (hands: TabHands): readonly TabEntry[] => [
  /* No key announced: this window has no close binding, and `shortcuts.test`
     is what caught the one written here from memory. A menu that promises a
     key nothing listens for is worse than a menu that promises none. */
  { label: 'Close', run: hands.close },
  { label: 'Close others', run: (id) => hands.sweep(id, 'others') },
  { rule: true },
  { label: 'Close to the right', run: (id) => hands.sweep(id, 'right') },
  { label: 'Close to the left', run: (id) => hands.sweep(id, 'left') },
  { rule: true },
  { label: 'Close all', bad: true, run: (id) => hands.sweep(id, 'all') },
]
