import { composing } from './typing'

/*
 * The keys a focused tile answers to, and the labels its menu shows for them.
 *
 * One table for both. The old card menu showed F2 and ⌘M beside entries that
 * did nothing, and a shortcut label the tile does not answer to is the menu
 * lying about the keyboard.
 */

export type TileAction = 'open' | 'rename' | 'moveTo' | 'archive'

const mac = (): boolean => typeof navigator !== 'undefined' && /Mac/.test(navigator.platform)

/** The label for an action, shown on the menu entry that performs it. */
export const keyLabel = (action: TileAction): string =>
  ({ open: '↵', rename: 'F2', moveTo: mac() ? '⌘M' : 'Ctrl+M', archive: 'Del' })[action]

/** What a key pressed on a focused tile asks for, or null. */
export function tileAction(event: {
  readonly key: string
  readonly ctrlKey?: boolean
  readonly metaKey?: boolean
  readonly isComposing?: boolean
  readonly nativeEvent?: { readonly isComposing?: boolean }
  readonly keyCode?: number
}): TileAction | null {
  if (composing(event)) return null
  if (event.key === 'Enter' || event.key === ' ') return 'open'
  if (event.key === 'F2') return 'rename'
  if (event.key === 'Delete') return 'archive'
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'm') return 'moveTo'
  return null
}
