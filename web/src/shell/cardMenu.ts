import type { SessionEntry } from './sessionMenu'
import { keyLabel } from './tileKeys'
import { moved } from './wsbrowse'

/*
 * A card's menu, as data, so `wired` can count it.
 *
 * It used to be seven labels that closed the menu and did nothing — the tile
 * published `data-card` and the menu read `data-id`, so even a wired entry
 * would have been handed an empty id.
 */

export interface CardHands {
  open: () => void
  rename: () => void
  /** Present when there is another lane to move to. */
  moveTo?: () => void
  /** Present when the card's lane runs a step. */
  play?: () => void
  terminal: () => void
  chat: () => void
  /** Present when the card has a checkout. */
  copyBranch?: () => void
  archive: () => void
  remove: () => void
}

export const cardMenu = (hands: CardHands): readonly SessionEntry[] => [
  { label: 'Open', key: keyLabel('open'), run: hands.open },
  { label: 'Rename', key: keyLabel('rename'), run: hands.rename },
  ...(hands.moveTo ? [{ label: 'Move to…', key: keyLabel('moveTo'), run: hands.moveTo }] : []),
  { rule: true },
  hands.play ? { label: 'Run step', run: hands.play } : { label: 'Open a terminal', run: hands.terminal },
  { label: 'Chat about this card', run: hands.chat },
  ...(hands.copyBranch ? [{ label: 'Copy branch', run: hands.copyBranch }] : []),
  { rule: true },
  { label: 'Archive', key: keyLabel('archive'), run: hands.archive },
  { label: 'Delete…', bad: true, run: hands.remove },
]

/** Which entry an arrow key, Home or End lands on from entry `at`, or null for
 *  a key the menu leaves alone. */
export function menuFocus(key: string, at: number, count: number): number | null {
  if (count === 0) return null
  if (key === 'Home') return 0
  if (key === 'End') return count - 1
  if (key === 'ArrowDown' || key === 'ArrowUp') return moved(at, key === 'ArrowDown' ? 1 : -1, count)
  return null
}
