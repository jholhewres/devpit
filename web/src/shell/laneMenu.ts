import type { SessionEntry } from './sessionMenu'

/*
 * A lane's menu, as data, so `wired` can count it.
 *
 * Renaming used to be a click on the name — on the same head that is the drag
 * handle — and deleting was an X that appeared on top of the step picker.
 */

export interface LaneHands {
  rename: () => void
  addCard: () => void
  shift: (by: -1 | 1) => void
  remove: () => void
}

export const laneMenu = (hands: LaneHands): readonly SessionEntry[] => [
  { label: 'Rename', run: hands.rename },
  { label: 'Add card', run: hands.addCard },
  { rule: true },
  { label: 'Move left', run: () => hands.shift(-1) },
  { label: 'Move right', run: () => hands.shift(1) },
  { rule: true },
  { label: 'Delete…', bad: true, run: hands.remove },
]
