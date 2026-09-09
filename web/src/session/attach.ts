import { Channel, invoke } from '@tauri-apps/api/core'
import { commands } from '../gen/bindings'

type GenerationHost = typeof globalThis & {
  __devpitAttachGeneration?: number
}

function nextClientId(): string {
  const host = globalThis as GenerationHost
  const floor = Date.now() * 1000
  host.__devpitAttachGeneration = Math.max(
    floor,
    (host.__devpitAttachGeneration ?? floor) + 1
  )
  return String(host.__devpitAttachGeneration).padStart(16, '0')
}

/**
 * The binary attach path. specta cannot describe `Channel<InvokeResponseBody>`,
 * so this wrapper is the one place the name `session_attach` is typed by hand.
 */
export function sessionAttach(
  projectId: string,
  paneId: string,
  rows: number,
  cols: number,
  onFrame: (bytes: Uint8Array) => void
): { stop: () => void; done: Promise<void> } {
  let cancelled = false
  const clientId = nextClientId()
  const onFrameChannel = new Channel<ArrayBuffer | number[]>()
  onFrameChannel.onmessage = (frame) => {
    if (cancelled) return
    const bytes = frame instanceof ArrayBuffer ? new Uint8Array(frame) : new Uint8Array(frame)
    onFrame(bytes)
  }

  const done = invoke('session_attach', {
    projectId,
    paneId,
    clientId,
    rows,
    cols,
    onFrame: onFrameChannel
  }).then(
    () => undefined,
    (thrown: unknown) => {
      if (cancelled) return
      throw thrown
    }
  )

  return {
    stop: () => {
      cancelled = true
      void commands.sessionDetach(paneId, clientId)
    },
    done
  }
}
