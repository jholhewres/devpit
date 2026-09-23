import { createContext, useCallback, useContext, useRef, useSyncExternalStore } from 'react'

import type { Shell } from './shape'
import { useShell } from './useShell'

/*
 * The shell, read one slice at a time.
 *
 * `useShell()` hands back the whole shell, so a component re-renders on every
 * change anywhere in it — a tab focused, a terminal's command changing, an
 * agent reporting. A link in a chat answer only needs `show`, and a long
 * answer has hundreds of links.
 *
 * `useShellPick(pick)` subscribes to a store the provider fills, and renders
 * again only when what `pick` returns changes — compared field by field, so a
 * pick that builds `{ show, project }` is stable while both are.
 */
export interface ShellStore {
  get(): Shell
  subscribe(listener: () => void): () => void
}

export const ShellStoreContext = createContext<ShellStore | null>(null)

/* A store holding the latest shell. `put` is called while the provider
   renders; `tell` once that render is committed. */
export function createShellStore(first: Shell): ShellStore & { put(next: Shell): void; tell(): void } {
  let current = first
  const listeners = new Set<() => void>()
  return {
    get: () => current,
    put: (next) => {
      current = next
    },
    tell: () => listeners.forEach((listener) => listener()),
    subscribe: (listener) => {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
  }
}

export function shallowEqual(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false
  const left = Object.keys(a)
  if (left.length !== Object.keys(b).length) return false
  return left.every((key) => Object.is((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key]))
}

export function useShellPick<T>(pick: (shell: Shell) => T): T {
  const store = useContext(ShellStoreContext)
  /* No store is a tree without the provider — a test that stands the shell
     in with a mock of `useShell`. It never changes for a mounted component,
     so the hooks below run in the same order on every render of it. */
  if (!store) return pick(useShell())
  return usePicked(store, pick)
}

function usePicked<T>(store: ShellStore, pick: (shell: Shell) => T): T {
  /* The latest `pick`, read on every snapshot: one written inline over a prop
     — a pane's id — is a new closure each render, and the snapshot has to
     pick with the prop as it is now. */
  const picking = useRef(pick)
  picking.current = pick
  const last = useRef<{ value: T } | null>(null)
  const snapshot = useCallback(() => {
    const next = picking.current(store.get())
    if (last.current && shallowEqual(last.current.value, next)) return last.current.value
    last.current = { value: next }
    return next
  }, [store])
  return useSyncExternalStore(store.subscribe, snapshot, snapshot)
}
