import { Channel, invoke } from '@tauri-apps/api/core'

import { inTauri } from './window'

/* The pty streams raw bytes over a Channel rather than JSON: a terminal that
   round-trips its output through UTF-8 JSON loses the bytes it exists to
   carry. These four are hand-written because specta cannot describe that. */
export interface Attached {
  write: (data: string) => void
  resize: (rows: number, cols: number) => Promise<{ rows: number; cols: number } | null>
  detach: () => void
}

export async function attach(
  projectId: string,
  paneId: string,
  clientId: string,
  size: { rows: number; cols: number },
  onFrame: (bytes: Uint8Array) => void,
): Promise<Attached | null> {
  if (!inTauri()) return null

  const channel = new Channel<ArrayBuffer | number[]>()
  channel.onmessage = (frame) => {
    onFrame(frame instanceof ArrayBuffer ? new Uint8Array(frame) : Uint8Array.from(frame))
  }

  await invoke('session_attach', {
    projectId,
    paneId,
    clientId,
    rows: size.rows,
    cols: size.cols,
    onFrame: channel,
  })

  return {
    write: (data) => void invoke('session_write', { projectId, paneId, data }),
    /* The pty clamps. A pane that believes it has two hundred columns when it
       has eighty draws wrongly a long way from the line that caused it, so the
       applied size is what the caller gets back. */
    resize: async (rows, cols) =>
      (await invoke('session_resize', { projectId, paneId, rows, cols })) as {
        rows: number
        cols: number
      } | null,
    detach: () => void invoke('session_detach', { projectId, paneId, clientId }),
  }
}

export async function scrollback(projectId: string, paneId: string): Promise<Uint8Array | null> {
  if (!inTauri()) return null
  const bytes = (await invoke('pane_scrollback', { projectId, paneId })) as number[] | null
  return bytes ? Uint8Array.from(bytes) : null
}
