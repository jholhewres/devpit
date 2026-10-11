import type { Press } from './keys'
import { keysLabel, matches, type ShortcutId } from './shortcuts'
import { composing } from './typing'

/*
 * The keys a focused tile answers to, and the labels its menu shows for them.
 *
 * One table for both, read from the registry. The old card menu showed F2 and
 * ⌘M beside entries that did nothing, and a shortcut label the tile does not
 * answer to is the menu lying about the keyboard.
 */

export type TileAction = 'open' | 'rename' | 'moveTo' | 'archive'

const SHORTCUT: Readonly<Record<TileAction, ShortcutId>> = {
  open: 'openCard',
  rename: 'rename',
  moveTo: 'moveCard',
  archive: 'archiveCard',
}

/** The label for an action, shown on the menu entry that performs it. */
export const keyLabel = (action: TileAction): string => keysLabel(SHORTCUT[action])

/** What a key pressed on a focused tile asks for, or null. */
export function tileAction(event: Press & {
  readonly isComposing?: boolean
  readonly nativeEvent?: { readonly isComposing?: boolean }
  readonly keyCode?: number
  readonly target?: unknown
  readonly currentTarget?: unknown
}): TileAction | null {
  if (composing(event)) return null
  /* A key on a button inside the tile is that button's: Enter on the play
     button plays, and Delete there is not an archive. */
  if (event.target !== event.currentTarget) return null
  if (matches('openCard', event)) return 'open'
  if (matches('rename', event)) return 'rename'
  if (matches('archiveCard', event)) return 'archive'
  if (matches('moveCard', event)) return 'moveTo'
  return null
}
