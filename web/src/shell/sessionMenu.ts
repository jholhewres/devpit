import type { Entry } from './fileMenu'

/*
 * The session row's context menu.
 *
 * Apart from the component for the same reason `fileMenu` is: a menu is data,
 * and a list of labels in the middle of a component is a list nothing can
 * test. The two differ in how they name the row they act on — a file row
 * publishes `data-path`, a session row publishes `data-id` — so this one
 * carries `run`, which is called with that id.
 */

export interface SessionEntry extends Entry {
  readonly run?: (id: string) => void
}

/** What the shell has to hand over for these to act. */
export interface SessionHands {
  focus: (id: string) => void
  close: (id: string) => void
  rename: (id: string) => void
}

export const sessionMenu = (hands: SessionHands): readonly SessionEntry[] => [
  { label: 'Open', key: '↵', run: hands.focus },
  { rule: true },
  { label: 'Rename', key: 'F2', run: hands.rename },
  { rule: true },
  { label: 'Close session', bad: true, run: hands.close },
]
