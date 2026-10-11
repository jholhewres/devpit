import { useSyncExternalStore } from 'react'

/*
 * The keys a person chose in place of the defaults, kept in this window's
 * storage like the rail's order and the tabs: a preference of this machine's
 * window, not a record. Read at every press, so a change works at once.
 */

const KEY = 'devpit:shortcuts'

export type Rebound = Readonly<Record<string, readonly string[]>>

function read(): Rebound {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(KEY) ?? '{}')
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {}
    return Object.fromEntries(
      Object.entries(raw).filter(
        (entry): entry is [string, string[]] => Array.isArray(entry[1]) && entry[1].every((one) => typeof one === 'string' && one.length > 0),
      ),
    )
  } catch {
    return {}
  }
}

let kept: Rebound = read()
let version = 0
const told = new Set<() => void>()

export const rebound = (): Rebound => kept

export function keepRebound(next: Rebound): void {
  kept = next
  version += 1
  try {
    localStorage.setItem(KEY, JSON.stringify(next))
  } catch {
    /* Not kept past this window; it still holds until it closes. */
  }
  for (const listener of told) listener()
}

function subscribe(listener: () => void): () => void {
  told.add(listener)
  return () => told.delete(listener)
}

/** Renders again when any key is rebound. */
export function useRebound(): number {
  return useSyncExternalStore(subscribe, () => version)
}
