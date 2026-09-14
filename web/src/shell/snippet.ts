/*
 * A search hit's words, with the matched ones marked.
 *
 * The index wraps each match in `[` and `]`. Split into pieces rather than
 * turned into markup: the text is a transcript's, and nothing in it is ever
 * handed to the page as HTML.
 */

export interface Piece {
  readonly text: string
  readonly hit: boolean
}

export function pieces(snippet: string): readonly Piece[] {
  const out: Piece[] = []
  const pattern = /\[([^\]]*)\]/g
  let at = 0
  for (const match of snippet.matchAll(pattern)) {
    const start = match.index ?? 0
    if (start > at) out.push({ text: snippet.slice(at, start), hit: false })
    if (match[1]) out.push({ text: match[1], hit: true })
    at = start + match[0].length
  }
  if (at < snippet.length) out.push({ text: snippet.slice(at), hit: false })
  return out
}
