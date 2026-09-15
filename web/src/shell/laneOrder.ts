/*
 * Where a dragged column lands.
 *
 * A function rather than a handler, because the rule is the interesting part
 * and a rule inside a pointer event is a rule no test can reach: dropping a
 * column on itself changes nothing, and dropping it past its own position has
 * to account for the gap it left behind.
 */

/** The ids in their new order, or the same list when nothing moved. */
export function reordered(
  ids: readonly string[],
  moving: string,
  onto: string,
): readonly string[] {
  if (moving === onto) return ids
  const from = ids.indexOf(moving)
  const to = ids.indexOf(onto)
  if (from < 0 || to < 0) return ids

  /* Into the list *without* the moved column, at the target's index in the
     list *with* it. Dropping a column on another means taking its place, and
     those two indices are what "its place" is: moving left, the target shifts
     right; moving right, taking the column out already closed the gap. */
  const rest = ids.filter((id) => id !== moving)
  return [...rest.slice(0, to), moving, ...rest.slice(to)]
}

/** The ids with one moved a place left (-1) or right (1), or the same list at an edge. */
export function shifted(ids: readonly string[], moving: string, by: -1 | 1): readonly string[] {
  const from = ids.indexOf(moving)
  const to = from + by
  if (from < 0 || to < 0 || to >= ids.length) return ids
  const next = [...ids]
  next[from] = ids[to]!
  next[to] = moving
  return next
}
