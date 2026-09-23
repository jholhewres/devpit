/*
 * `@file` in the composer: a path from the project, offered as it is typed.
 *
 * The CLI reads `@path` in a prompt as the file itself, so a mention is only
 * text — picked from the index rather than typed from memory. Attachments do
 * the same for a file dropped from outside; this is the one already here.
 */

export interface Mention {
  /** Where the `@` is. */
  readonly start: number
  /** What was typed after it. */
  readonly query: string
}

/* The `@word` the caret is at the end of — begun at the start of the text or
   after a space, so an address like `me@host` is not taken for one. */
export function mentionAt(prompt: string, caret: number): Mention | null {
  const before = prompt.slice(0, caret)
  const match = /(^|\s)@([^\s@]*)$/.exec(before)
  if (!match) return null
  return { start: caret - match[2]!.length - 1, query: match[2]! }
}

/* The text with the mention replaced by the path, and a space so the sentence
   goes on. Returns where the caret belongs too. */
export function mentioned(prompt: string, at: Mention, caret: number, path: string): { text: string; caret: number } {
  const inserted = `@${path} `
  const text = prompt.slice(0, at.start) + inserted + prompt.slice(caret).replace(/^ /, '')
  return { text, caret: at.start + inserted.length }
}
