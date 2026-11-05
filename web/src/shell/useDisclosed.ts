import { useCallback, useEffect, useState } from 'react'

/*
 * Whether a fold is open, kept by its key outside the row that draws it: a
 * turn scrolled out of the thread's window is unmounted, and the fold a
 * person opened in it is open again when it comes back.
 */

/* Folds remembered at most; the oldest go first. */
const MOST = 512
const kept = new Map<string, boolean>()
/* The folds drawn now, told when something outside them opens one. */
const drawn = new Map<string, Set<(open: boolean) => void>>()

/* Opens the fold `key`, drawn or not: a search that lands in it shows what it found. */
export function disclose(key: string): void {
  kept.delete(key)
  kept.set(key, true)
  if (kept.size > MOST) kept.delete(kept.keys().next().value!)
  for (const tell of drawn.get(key) ?? []) tell(true)
}

export function useDisclosed(key: string | null, initial: boolean): [boolean, (open: boolean | ((was: boolean) => boolean)) => void] {
  const [open, setOpen] = useState(() => (key !== null ? (kept.get(key) ?? initial) : initial))
  const set = useCallback(
    (next: boolean | ((was: boolean) => boolean)) =>
      setOpen((was) => {
        const now = typeof next === 'function' ? next(was) : next
        if (key !== null) {
          kept.delete(key)
          kept.set(key, now)
          if (kept.size > MOST) kept.delete(kept.keys().next().value!)
        }
        return now
      }),
    [key],
  )
  useEffect(() => {
    if (key === null) return
    const tells = drawn.get(key) ?? new Set<(open: boolean) => void>()
    tells.add(setOpen)
    drawn.set(key, tells)
    return () => {
      tells.delete(setOpen)
      if (tells.size === 0) drawn.delete(key)
    }
  }, [key])
  return [open, set]
}
