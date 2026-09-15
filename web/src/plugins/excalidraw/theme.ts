import type { Theme } from '../../shell/shape'

/** Excalidraw only knows two themes; `'system'` borrows the OS's. */
export function excalidrawTheme(pref: Theme, systemDark: boolean): 'light' | 'dark' {
  if (pref === 'system') return systemDark ? 'dark' : 'light'
  return pref
}
