import { useSyncExternalStore } from 'react'

import { inOrder } from './inOrder'
import { ask, commands } from './live'

/*
 * The keys a person chose in place of the defaults, kept in the app's
 * settings with its other preferences. Held here as well and read at every
 * press, so a change works at once and a press never waits on a command.
 */

/* Where this window kept them before they moved to the settings. */
const LEGACY = 'devpit:shortcuts'

export type Rebound = Readonly<Record<string, readonly string[]>>

/** Only the entries that are a list of keys: the value came from storage. */
function reboundOf(raw: unknown): Rebound {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {}
  return Object.fromEntries(
    Object.entries(raw).filter(
      (entry): entry is [string, string[]] => Array.isArray(entry[1]) && entry[1].every((one) => typeof one === 'string' && one.length > 0),
    ),
  )
}

function legacy(): Rebound | null {
  try {
    const raw = localStorage.getItem(LEGACY)
    return raw === null ? null : reboundOf(JSON.parse(raw))
  } catch {
    return null
  }
}

function forgetLegacy(): void {
  try {
    localStorage.removeItem(LEGACY)
  } catch {
    /* Read again next start, and dropped then: the settings have it. */
  }
}

/* The window's own copy, until the settings answer: the first start after
   the move must not open on the defaults. */
let kept: Rebound = legacy() ?? {}
let version = 0
const told = new Set<() => void>()

export const rebound = (): Rebound => kept

function hold(next: Rebound): void {
  kept = next
  version += 1
  for (const listener of told) listener()
}

const write = (next: Rebound) => {
  const keys = Object.fromEntries(Object.entries(next).map(([id, said]) => [id, [...said]]))
  return inOrder('rebound', () => ask(() => commands.reboundSet({ keys })))
}

export function keepRebound(next: Rebound): void {
  hold(next)
  /* A failed write still holds until the window closes. */
  void write(next)
}

/** Which keys to start with, and whether the window's own copy moves to the settings. */
function settled(stored: Rebound, local: Rebound | null): { keys: Rebound; move: boolean } {
  /* Stored already means a move that wrote and never got to forget. */
  if (local === null || Object.keys(stored).length > 0) return { keys: stored, move: false }
  return { keys: local, move: Object.keys(local).length > 0 }
}

/** Reads the kept keys back as the window starts, moving this window's own copy to the settings once. */
export async function loadRebound(): Promise<void> {
  const before = version
  const asked = await ask(() => commands.reboundRead())
  /* A key recorded meanwhile is newer than anything read, and is being written. */
  if (!asked.data || version !== before) return
  const local = legacy()
  const { keys, move } = settled(reboundOf(asked.data.keys), local)
  hold(keys)
  if (!move) {
    if (local !== null) forgetLegacy()
    return
  }
  if ((await write(keys)).data) forgetLegacy()
}

function subscribe(listener: () => void): () => void {
  told.add(listener)
  return () => told.delete(listener)
}

/** Renders again when any key is rebound. */
export function useRebound(): number {
  return useSyncExternalStore(subscribe, () => version)
}
