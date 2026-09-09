import { getCurrentWindow } from '@tauri-apps/api/window'

/*
 * The window, for a window that draws its own frame.
 *
 * `decorations: false` buys the titlebar back as ours to design, and hands
 * back the bill: moving, resizing, minimising, maximising and closing all stop
 * being the system's job. Every one of them is wired here.
 *
 * Outside Tauri — `vite preview`, the tests — these are no-ops rather than
 * throws, so the same shell renders in a plain browser.
 */
export const inTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const onWindow = async (act: (win: ReturnType<typeof getCurrentWindow>) => Promise<unknown>): Promise<void> => {
  if (!inTauri()) return
  try {
    await act(getCurrentWindow())
  } catch {
    /* The window went away mid-gesture; there is nothing to report. */
  }
}

export const minimize = (): void => void onWindow((win) => win.minimize())
export const toggleMaximize = (): void => void onWindow((win) => win.toggleMaximize())
export const close = (): void => void onWindow((win) => win.close())

/** The eight directions a frameless window can be pulled by. */
export const EDGES = [
  'North',
  'South',
  'East',
  'West',
  'NorthEast',
  'NorthWest',
  'SouthEast',
  'SouthWest',
] as const

export type Edge = (typeof EDGES)[number]

/**
 * Hands the drag to the window manager.
 *
 * It has to start on pointerdown, not on click: the compositor takes over the
 * pointer for the rest of the gesture, and a click never arrives.
 */
export const startResize = (edge: Edge): void =>
  void onWindow((win) => win.startResizeDragging(edge))

/** True while the window fills the screen, so the corners can square off. */
export async function isMaximized(): Promise<boolean> {
  if (!inTauri()) return false
  try {
    return await getCurrentWindow().isMaximized()
  } catch {
    return false
  }
}

/** Calls back whenever the window is resized, so the corners keep up. */
export function onResized(then: () => void): () => void {
  if (!inTauri()) return () => {}
  let drop: (() => void) | undefined
  void getCurrentWindow()
    .onResized(() => then())
    .then((unlisten) => {
      drop = unlisten
    })
  return () => drop?.()
}
