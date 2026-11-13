import { useCallback, useRef, useSyncExternalStore } from 'react'

/*
 * A slice of a store, read with `useSyncExternalStore`: a component renders
 * again only when what `pick` returns changes, compared field by field (an
 * array item by item), so a pick that builds `{ a, b }` is stable while both are.
 */
export interface Store<S> {
  get(): S
  subscribe(listener: () => void): () => void
}

export function shallowEqual(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false
  const left = Object.keys(a)
  if (left.length !== Object.keys(b).length) return false
  return left.every((key) => Object.is((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key]))
}

export function usePicked<S, T>(store: Store<S>, pick: (state: S) => T): T {
  /* The latest `pick`, read on every snapshot: one written inline over a prop
     is a new closure each render, and the snapshot has to pick with the prop
     as it is now. */
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
