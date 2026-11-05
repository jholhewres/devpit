import { Channel, invoke } from '@tauri-apps/api/core'

import { commands } from '../gen/bindings'
import { inBrowser } from '../transport'
import { LOST, PaneSocket, UNAVAILABLE, type PaneListener } from './paneSocket'
import { reason } from './reason'
import { inTauri } from './window'

/* One live pane, whichever way its bytes travel. */
export interface Pane {
  write: (data: string) => void
  resize: (rows: number, cols: number) => Promise<{ rows: number; cols: number } | null>
  /** Parsed bytes, so the pty reads on. */
  ack: (bytes: number) => void
  /** The page of history before what this attach holds, where there is one. */
  older: () => Promise<{ left: number; bytes: Uint8Array } | null>
  detach: () => void
}

/* Desktop: the URL with its ticket, asked of the window's own IPC once. A
   browser signed in by cookie opens `/panes` on the page's own host. `null`
   when the loopback server is not running: the panes fall back to the
   channel. */
let socket: Promise<PaneSocket | null> | null = null

function paneSocket(): Promise<PaneSocket | null> {
  socket ??= (async () => {
    if (inBrowser()) return new PaneSocket(async () => new WebSocket(`ws://${window.location.host}/panes`))
    if (!inTauri()) return null
    const url = (await invoke('panes_socket').catch(() => null)) as string | null
    return url ? new PaneSocket(async () => new WebSocket(url)) : null
  })()
  return socket
}

/**
 * Attaches `paneId`: over the panes socket when there is one, else over the
 * channel, with its scrollback first. The replay of a socket attach comes in
 * the stream itself, before the first live byte.
 */
export async function attachPane(
  projectId: string,
  paneId: string,
  size: { rows: number; cols: number },
  on: PaneListener,
): Promise<Pane | null> {
  const panes = await paneSocket()
  if (panes)
    return panes.attach(projectId, paneId, size, {
      ...on,
      end: (why) => {
        /* Refused (a policy, a server gone): the channel from here on. A
           browser has none, and keeps the socket to try again. */
        if (why === UNAVAILABLE && !inBrowser()) socket = Promise.resolve(null)
        on.end(why)
      },
    })
  const past = await scrollback(paneId)
  if (past) on.output(new TextEncoder().encode(past), true)
  const attached = attach(projectId, paneId, size, (bytes) => on.output(bytes, false), on.end)
  return attached && { ...attached, ack: () => undefined, older: async () => null }
}

/**
 * How long to wait before attaching again after an attach ended with
 * `failed`; `null` when attaching again would not help. `tries` counts the
 * attempts since the pane last drew.
 */
export function retryIn(failed: string | null, tries: number): number | null {
  if (failed === LOST) return 1000
  if (failed !== UNAVAILABLE) return null
  /* The window has the channel to fall back on at once; a browser waits for
     the server, longer each time. */
  return inBrowser() ? Math.min(1000 * 2 ** tries, 15_000) : 0
}

/* The pty streams raw bytes over a Channel rather than JSON: a terminal that
   round-trips its output through UTF-8 JSON loses the bytes it exists to
   carry. These four are hand-written because specta cannot describe that. */
export interface Attached {
  write: (data: string) => void
  resize: (rows: number, cols: number) => Promise<{ rows: number; cols: number } | null>
  detach: () => void
}

/* `session_attach` runs the frame loop inside the command, so its promise is
   the pane's *lifetime*, not its readiness: it settles when the pty ends.
   Awaiting it here left the caller without a handle for as long as the
   terminal worked, which is exactly when the handle is wanted — nothing was
   ever wired to `onData`, so the pane took no typing at all. The promise is
   kept for the end, and the handle is returned at once. */
export function attach(
  projectId: string,
  paneId: string,
  size: { rows: number; cols: number },
  onFrame: (bytes: Uint8Array) => void,
  onEnd?: (error: string | null) => void,
): Attached | null {
  /* Its bytes stream over a Tauri channel, which a browser does not have. */
  if (!inTauri() || inBrowser()) return null

  /* One id per attach, not one per window. Two mounts of the same pane under
     one id are indistinguishable to the backend: the older one's detach
     arrives and takes the pane from the newer one, which is what a remount
     is. It is also what tells a late detach apart from a live one. */
  const clientId = crypto.randomUUID()

  const channel = new Channel<ArrayBuffer | number[]>()
  channel.onmessage = (frame) => {
    onFrame(frame instanceof ArrayBuffer ? new Uint8Array(frame) : Uint8Array.from(frame))
  }

  void invoke('session_attach', {
    projectId,
    paneId,
    clientId,
    rows: size.rows,
    cols: size.cols,
    onFrame: channel,
  }).then(
    () => onEnd?.(null),
    /* An `RpcError` is a plain object, so `String` on it says `[object
       Object]` — which is what the pane printed instead of the reason it
       could not attach. See `reason`. */
    (error: unknown) => onEnd?.(reason(error, 'the terminal could not be attached')),
  )

  return {
    /* A pane is addressed by itself. These three carried a `projectId` the
       commands never declared, so Tauri dropped it — harmless, and a lie
       about what identifies a pane. */
    /* Typing into a pane whose pty has ended is refused with `not_found`;
       the end itself reaches `onEnd`, so the keystroke has nothing to add. */
    write: (data) => void invoke('session_write', { paneId, data }).catch(() => undefined),
    /* The pty clamps. A pane that believes it has two hundred columns when it
       has eighty draws wrongly a long way from the line that caused it, so the
       applied size is what the caller gets back. */
    resize: async (rows, cols) =>
      (await invoke('session_resize', { paneId, rows, cols })) as {
        rows: number
        cols: number
      } | null,
    // A pane already gone has nothing left to detach from.
    detach: () => void invoke('session_detach', { paneId, clientId }).catch(() => undefined),
  }
}

/*
 * What this pane printed before now.
 *
 * Through the generated contract rather than a hand-written `invoke`: this
 * asked for `number[]`, `pane.scrollback` answers `{ text, truncated }`, and
 * `Uint8Array.from` on an object with no length is an empty array rather than
 * a throw. So every reopened terminal silently came back blank, and nothing
 * anywhere said why.
 */
export async function scrollback(paneId: string): Promise<string | null> {
  if (!inTauri()) return null
  const answer = await commands.paneScrollback(paneId)
  return answer.status === 'ok' ? answer.data.text : null
}
