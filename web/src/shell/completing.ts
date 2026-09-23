/*
 * Tab in the terminal's editor: the word under the cursor, finished.
 */

/** The word the cursor ends: where it starts, and what it is so far. */
export function wordAt(text: string, caret: number): { start: number; word: string } {
  const before = text.slice(0, caret)
  const start = Math.max(before.lastIndexOf(' '), before.lastIndexOf('\n'), before.lastIndexOf('\t')) + 1
  return { start, word: before.slice(start) }
}

/** What every completion begins with — as far as Tab can go without choosing. */
export function common(found: readonly string[]): string {
  if (found.length === 0) return ''
  let prefix = found[0]!
  for (const one of found.slice(1)) {
    let at = 0
    while (at < prefix.length && at < one.length && prefix[at] === one[at]) at++
    prefix = prefix.slice(0, at)
  }
  return prefix
}

/** The text with the word at `start` replaced, and where the cursor lands. */
export function finished(text: string, start: number, caret: number, word: string): { text: string; caret: number } {
  return { text: text.slice(0, start) + word + text.slice(caret), caret: start + word.length }
}
