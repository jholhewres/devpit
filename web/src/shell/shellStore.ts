import { createContext, useContext } from 'react'

import { usePicked, type Store } from './picked'
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
export type ShellStore = Store<Shell>

export { shallowEqual } from './picked'

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

export function useShellPick<T>(pick: (shell: Shell) => T): T {
  const store = useContext(ShellStoreContext)
  /* No store is a tree without the provider — a test that stands the shell
     in with a mock of `useShell`. It never changes for a mounted component,
     so the hooks below run in the same order on every render of it. */
  if (!store) return pick(useShell())
  return usePicked(store, pick)
}

/* What the window's frame reads: a slice, not the whole shell — it draws
   every other component, and re-rendering it for each agent report re-drew
   the whole window. */
export function useWindowShell() {
  return useShellPick((all) => ({
    side: all.side,
    files: all.files,
    widths: all.widths,
    signedIn: all.signedIn,
    palette: all.palette,
    openPalette: all.openPalette,
    closePalette: all.closePalette,
    show: all.show,
    theme: all.theme,
    openPrefs: all.openPrefs,
    closePrefs: all.closePrefs,
    managing: all.managing,
    closeManager: all.closeManager,
  }))
}
