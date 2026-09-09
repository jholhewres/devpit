/* How much room something takes, in words rather than digits.

   Rounded to one decimal from MB up: `1.4 GB` is the number a person acts on,
   and `1503238553 bytes` is not. */
const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'] as const

export function bytes(count: number | null): string | null {
  /* Null and zero are different: null is "we could not read it", and drawing
     0 B for it would be a claim we cannot make. */
  if (count === null) return null
  if (count === 0) return null
  let size = count
  let unit = 0
  while (size >= 1024 && unit < UNITS.length - 1) {
    size /= 1024
    unit += 1
  }
  return `${unit < 2 ? Math.round(size) : size.toFixed(1)} ${UNITS[unit]}`
}
