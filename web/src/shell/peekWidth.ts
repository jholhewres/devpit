/* How wide a document read beside the chat is: the person's choice, within
   room for both. */

export const PEEK_DEFAULT = 520
const PEEK_MIN = 320
/* What the chat keeps, however wide the document is pulled. */
const CHAT_MIN = 360
const KEY = 'devpit:peek-width'

export function peekWidth(wanted: number, pane: number): number {
  const most = Math.max(PEEK_MIN, pane - CHAT_MIN)
  return Math.round(Math.min(most, Math.max(PEEK_MIN, wanted)))
}

export function keptWidth(): number {
  try {
    const kept = Number(localStorage.getItem(KEY))
    return kept > 0 ? kept : PEEK_DEFAULT
  } catch {
    return PEEK_DEFAULT
  }
}

export function keepWidth(width: number): void {
  try {
    localStorage.setItem(KEY, String(width))
  } catch {
    /* Not kept: the next one opens at the default. */
  }
}
