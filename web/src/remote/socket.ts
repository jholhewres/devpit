import type { RemoteIn, RemoteOut } from '../gen/bindings'

/*
 * The one WebSocket to the machine: hello with the device's token, then
 * messages both ways. Dropped — the machine restarted, the phone slept, the
 * device was changed on the machine — it connects again, a little later each
 * time, and says hello again.
 */

const TOKEN = 'devpit.remote.token'

export function tokenKept(): string | null {
  try {
    return localStorage.getItem(TOKEN)
  } catch {
    return null
  }
}

export function keepToken(token: string | null): void {
  try {
    if (token) localStorage.setItem(TOKEN, token)
    else localStorage.removeItem(TOKEN)
  } catch {
    // Unkept, the device pairs again next time.
  }
}

export interface Link {
  send: (message: RemoteIn) => void
  close: () => void
}

/** The wait before connecting again, after `tries` failed ones. */
export function backoff(tries: number): number {
  return Math.min(15_000, 500 * 2 ** Math.min(tries, 5)) + Math.round(Math.random() * 250)
}

export function connect(token: string, heard: (message: RemoteOut) => void, state: (up: boolean) => void): Link {
  let socket: WebSocket | null = null
  let tries = 0
  let closed = false
  let again: number | undefined
  let beat: number | undefined

  const open = (): void => {
    const scheme = location.protocol === 'https:' ? 'wss:' : 'ws:'
    socket = new WebSocket(`${scheme}//${location.host}/api/ws`)
    socket.onopen = () => {
      tries = 0
      socket?.send(JSON.stringify({ t: 'hello', token } satisfies RemoteIn))
      state(true)
      beat = window.setInterval(() => socket?.send(JSON.stringify({ t: 'ping' } satisfies RemoteIn)), 30_000)
    }
    socket.onmessage = (event) => {
      try {
        heard(JSON.parse(String(event.data)) as RemoteOut)
      } catch {
        // Not one of the machine's: left alone.
      }
    }
    socket.onclose = () => {
      window.clearInterval(beat)
      state(false)
      if (closed) return
      again = window.setTimeout(open, backoff(tries++))
    }
  }
  open()

  return {
    send: (message) => {
      if (socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify(message))
    },
    close: () => {
      closed = true
      window.clearTimeout(again)
      window.clearInterval(beat)
      socket?.close()
    },
  }
}

/** Pairs this device with the code off the machine's screen; the token, or why not. */
export async function pair(code: string, name: string): Promise<{ token: string } | { error: string }> {
  try {
    const answer = await fetch('/api/pair', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ code, name }),
    })
    const said = (await answer.json()) as { token?: string; error?: string }
    if (said.token) return { token: said.token }
    return { error: said.error ?? 'the machine did not pair it' }
  } catch {
    return { error: 'the machine could not be reached' }
  }
}

/** Bytes as base64, the way keys travel. */
export function base64Of(text: string): string {
  const bytes = new TextEncoder().encode(text)
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary)
}

export function bytesOf(b64: string): Uint8Array {
  const binary = atob(b64)
  const bytes = new Uint8Array(binary.length)
  for (let at = 0; at < binary.length; at++) bytes[at] = binary.charCodeAt(at)
  return bytes
}
