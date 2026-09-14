import { listen } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { getCurrentWindow } from '@tauri-apps/api/window'

import type { Happening, Question } from '../gen/bindings'

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
/* Whether files are being dragged over the window, from the webview's own
   drag events. A function over the event type so the rule is testable: a
   drop and a leave both end the hover. */
export function draggingAfter(type: string): boolean {
  return type === 'enter' || type === 'over'
}

/**
 * Calls back when files start or stop being dragged over the window.
 *
 * The drop itself lands on the window, not on an element, so there is no DOM
 * event to hang a target on — the pane draws its target from this.
 */
export function onFilesDragging(then: (over: boolean) => void): () => void {
  if (!inTauri()) return () => {}
  let dropped = false
  let drop: (() => void) | undefined
  void getCurrentWebview()
    .onDragDropEvent((event) => then(draggingAfter(event.payload.type)))
    .then((unlisten) => {
      if (dropped) unlisten()
      else drop = unlisten
    })
  return () => {
    dropped = true
    drop?.()
  }
}

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

/**
 * Calls back when an agent asks to be allowed to do something.
 *
 * Every conversation hears every question; matching it to the session that
 * raised it is the caller's job, because only the caller knows which session
 * it is showing.
 */
export function onPermissionAsked(then: (question: Question) => void): () => void {
  if (!inTauri()) return () => {}
  let dropped = false
  let drop: (() => void) | undefined
  void listen<Question>('permission:asked', (event) => then(event.payload)).then((unlisten) => {
    if (dropped) unlisten()
    else drop = unlisten
  })
  return () => {
    dropped = true
    drop?.()
  }
}

/**
 * Calls back whenever a pane says something about itself.
 *
 * One event for all of it — a prompt, a command starting, an exit code, a
 * title, a working directory — because they are all "something happened in
 * this pane", and five subscriptions to one question would leave the screen
 * ordering them itself. The payload names the pane, so a listener that cares
 * about one filters.
 *
 * A pane reports whether or not anyone is attached to it: see
 * `apps/desktop/src/tap.rs`.
 */
export function onHappening(then: (happening: Happening) => void): () => void {
  if (!inTauri()) return () => {}
  let dropped = false
  let drop: (() => void) | undefined
  void listen<Happening>('terminal:happening', (event) => then(event.payload)).then((unlisten) => {
    if (dropped) unlisten()
    else drop = unlisten
  })
  return () => {
    dropped = true
    drop?.()
  }
}

/*
 * Any event the backend emits, with no payload to read.
 *
 * `onHappening` is the shape for one that carries something; this is the
 * shape for one that only says "look again". The bell is the first of those:
 * the count and the list come from a command, and the event exists only to
 * say when to ask for them.
 */
export function onEvent(name: string, then: () => void): () => void {
  if (!inTauri()) return () => {}
  let dropped = false
  let drop: (() => void) | undefined
  void listen(name, () => then()).then((unlisten) => {
    if (dropped) unlisten()
    else drop = unlisten
  })
  return () => {
    dropped = true
    drop?.()
  }
}

/*
 * An event that names the thing it happened to.
 *
 * `onEvent` is for one that only says "look again"; this is for one that says
 * which card, or which run. The payload is a plain value rather than a shape,
 * because these two carry exactly that — `run:changed` is a card id, and
 * `run:progress` is a pair.
 */
export function onCarried<T>(name: string, then: (payload: T) => void): () => void {
  if (!inTauri()) return () => {}
  let dropped = false
  let drop: (() => void) | undefined
  void listen<T>(name, (event) => then(event.payload)).then((unlisten) => {
    if (dropped) unlisten()
    else drop = unlisten
  })
  return () => {
    dropped = true
    drop?.()
  }
}
