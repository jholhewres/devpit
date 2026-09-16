/*
 * The keys the window listens for, in one list.
 *
 * Announced in one file and handled in another is how ⌘P came to be printed
 * on a button that did nothing: the title said the key existed and no listener
 * had ever heard of it. `shortcuts.test.ts` reads the announcements off the
 * screens and compares them with this, so the next one cannot drift quietly.
 */

/** What a press means, when it means anything. */
export type Shortcut =
  | 'palette'
  | 'terminal'
  | 'chat'
  | 'project'
  | 'settings'
  | 'splitRight'
  | 'splitDown'
  | 'focus'
  | 'next'

/** How each one is written where it is announced, exactly as a title prints it. */
export const SHORTCUTS: Readonly<Record<Shortcut, string>> = {
  palette: '⌘K',
  terminal: '⌘T',
  chat: '⌘N',
  project: '⌘P',
  settings: '⌘,',
  splitRight: '⇧⌘D',
  splitDown: '⇧⌘E',
  focus: '⇧⌘F',
  next: '⇧⌘N',
}

const PLAIN: Readonly<Record<string, Shortcut>> = {
  k: 'palette',
  t: 'terminal',
  n: 'chat',
  p: 'project',
  ',': 'settings',
}

const SHIFTED: Readonly<Record<string, Shortcut>> = {
  d: 'splitRight',
  e: 'splitDown',
  f: 'focus',
  n: 'next',
}

/** A press, as much of one as this needs to know. */
export type Press = Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'shiftKey'>

export function shortcutFor(event: Press): Shortcut | null {
  // Ctrl stands in for ⌘ off macOS, and the window runs on both.
  if (!(event.metaKey || event.ctrlKey)) return null
  const key = event.key.toLowerCase()
  return (event.shiftKey ? SHIFTED[key] : PLAIN[key]) ?? null
}
