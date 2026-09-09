/*
 * How the search field ranks what it finds.
 *
 * The last segment of a path is what people type — `main.rs`, not
 * `apps/desktop/src/main.rs` — so a match there outranks one in a folder
 * name, and a prefix outranks a match in the middle.
 */

export interface Found<T> {
  readonly row: T
  readonly score: number
}

/** Where the text matched, as a score. Zero means it did not. */
export function score(text: string, query: string): number {
  if (query === '') return 1
  const haystack = text.toLowerCase()
  const needle = query.toLowerCase()
  const at = haystack.indexOf(needle)
  if (at < 0) return 0

  const tail = haystack.slice(haystack.lastIndexOf('/') + 1)
  const inTail = tail.indexOf(needle)

  if (inTail === 0) return 4
  if (inTail > 0) return 3
  if (at === 0) return 2
  return 1
}

/* Best first, and ties keep the order they came in — which for files is the
   sorted order the index was built in, so the list does not reshuffle. */
export function ranked<T>(rows: readonly T[], query: string, text: (row: T) => string, most = 8): T[] {
  return rows
    .map((row, at) => ({ row, at, score: score(text(row), query) }))
    .filter((found) => found.score > 0)
    .sort((a, b) => b.score - a.score || a.at - b.at)
    .slice(0, most)
    .map((found) => found.row)
}
