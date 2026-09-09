import { getCurrentWebview } from '@tauri-apps/api/webview'
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
  /* Registering is asynchronous, so the caller can be gone before the
     listener exists. Dropping it then has to be remembered, or the listener
     arrives with nobody left to unsubscribe it. */
  let dropped = false
  let drop: (() => void) | undefined
  void getCurrentWindow()
    .onResized(() => then())
    .then((unlisten) => {
      if (dropped) unlisten()
      else drop = unlisten
    })
  return () => {
    dropped = true
    drop?.()
  }
}

/**
 * Calls back with the paths of files dropped on the window.
 *
 * The webview's own drop event carries a `File` with no path, which is no use
 * to an agent that has to open it. Tauri's event carries the real paths.
 */
export function onFilesDropped(then: (paths: readonly string[]) => void): () => void {
  if (!inTauri()) return () => {}
  let dropped = false
  let drop: (() => void) | undefined
  void getCurrentWebview()
    .onDragDropEvent((event) => {
      if (event.payload.type === 'drop') then(event.payload.paths)
    })
    .then((unlisten) => {
      if (dropped) unlisten()
      else drop = unlisten
    })
  return () => {
    dropped = true
    drop?.()
  }
}
