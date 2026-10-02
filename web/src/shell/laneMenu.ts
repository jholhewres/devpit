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
  /** Says what the lane is for, or null to read it from its name again. */
  role?: (role: string | null) => void
}

/** What a lane can be for, in the words the menu uses. */
export const ROLES: readonly { role: string; label: string }[] = [
  { role: 'backlog', label: 'Holds waiting work' },
  { role: 'doing', label: 'Holds work in progress' },
  { role: 'check', label: 'Holds work to check' },
  { role: 'done', label: 'Holds done work' },
]

/* What a lane is for is how an agent knows where its card goes, whatever the
   lane is called. Read from the name until somebody says; the one in force
   is marked, and picking the one somebody chose goes back to the name. */
const roleEntries = (hands: LaneHands, now: { role: string | null; chosen: boolean }): SessionEntry[] =>
  hands.role
    ? [
        { rule: true },
        ...ROLES.map(({ role, label }) => ({
          label: `${now.role === role ? '✓ ' : ''}${label}${now.role === role && !now.chosen ? ' · by its name' : ''}`,
          run: () => hands.role?.(now.role === role && now.chosen ? null : role),
        })),
      ]
    : []

export const laneMenu = (
  hands: LaneHands,
  now: { role: string | null; chosen: boolean } = { role: null, chosen: false },
): readonly SessionEntry[] => [
  { label: 'Rename', run: hands.rename },
  { label: 'Add card', run: hands.addCard },
  { rule: true },
  { label: 'Move left', run: () => hands.shift(-1) },
  { label: 'Move right', run: () => hands.shift(1) },
  ...roleEntries(hands, now),
  { rule: true },
  { label: 'Delete…', bad: true, run: hands.remove },
]
