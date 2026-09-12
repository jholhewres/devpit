import { Channel, invoke } from '@tauri-apps/api/core'

import { commands } from '../gen/bindings'
import { reason } from './reason'
import { inTauri } from './window'

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
  if (!inTauri()) return null

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
    write: (data) => void invoke('session_write', { paneId, data }),
    /* The pty clamps. A pane that believes it has two hundred columns when it
       has eighty draws wrongly a long way from the line that caused it, so the
       applied size is what the caller gets back. */
    resize: async (rows, cols) =>
      (await invoke('session_resize', { paneId, rows, cols })) as {
        rows: number
        cols: number
      } | null,
    detach: () => void invoke('session_detach', { paneId, clientId }),
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
