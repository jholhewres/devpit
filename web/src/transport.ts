/*
 * The way to the backend, in the desktop window or in a browser on loopback.
 *
 * In the window, Tauri puts `__TAURI_INTERNALS__` on the page and every call
 * goes through its IPC. In a browser at `127.0.0.1:<port>` there is no Tauri,
 * so this puts a small stand-in there before anything else runs: a command is
 * `POST /rpc/<command>` and an event arrives on the `/events` socket. The
 * generated bindings and every `listen` keep calling what they always called,
 * and neither knows which one answered.
 *
 * The browser's only credential is the cookie the server set at sign-in; the
 * desktop's invoke key never reaches this page.
 */

type Callback = (payload: unknown) => void

type Internals = {
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>
  transformCallback: (callback: Callback, once?: boolean) => number
  unregisterCallback: (id: number) => void
  convertFileSrc: (path: string) => string
  metadata: { currentWindow: { label: string }; currentWebview: { windowLabel: string; label: string } }
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: Internals
    __DEVPIT_BROWSER__?: boolean
  }
}

/** True in a browser at the loopback server, false in the desktop window. */
export const inBrowser = (): boolean => typeof window !== 'undefined' && window.__DEVPIT_BROWSER__ === true

/** What `/rpc` answered, as the bindings expect an invoke to settle. */
export async function rpc(command: string, args: unknown, send: typeof fetch = fetch): Promise<unknown> {
  const answer = await send(`/rpc/${command}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    credentials: 'same-origin',
    body: JSON.stringify(args ?? {}),
  })
  const text = await answer.text()
  if (answer.ok) return text === '' ? null : JSON.parse(text)
  /* The command's own error (500, JSON) as the window would have rejected
     with it; the server's own refusals are plain text. */
  throw (answer.status === 500 && parsed(text)) || { message: text || answer.statusText, status: answer.status }
}

function parsed(text: string): unknown {
  try {
    return JSON.parse(text) as unknown
  } catch {
    return undefined
  }
}

/** The events a page subscribed to, kept to say again after a reconnect. */
export class Events {
  private readonly listeners = new Map<string, Map<number, Callback>>()
  private socket: WebSocket | null = null
  private next = 1
  private readonly open: () => WebSocket

  constructor(open: () => WebSocket) {
    this.open = open
  }

  listen(event: string, handler: Callback): number {
    const id = this.next++
    const forEvent = this.listeners.get(event) ?? new Map<number, Callback>()
    forEvent.set(id, handler)
    this.listeners.set(event, forEvent)
    this.say()
    return id
  }

  unlisten(event: string, id: number): void {
    const forEvent = this.listeners.get(event)
    forEvent?.delete(id)
    if (forEvent?.size === 0) this.listeners.delete(event)
    this.say()
  }

  /** One frame from the socket, handed to each listener of its event. */
  heard(frame: string): void {
    let said: { t?: unknown; p?: unknown }
    try {
      said = JSON.parse(frame) as { t?: unknown; p?: unknown }
    } catch {
      return
    }
    if (typeof said.t !== 'string') return
    for (const [id, handler] of this.listeners.get(said.t) ?? []) handler({ event: said.t, id, payload: said.p })
  }

  /** Tells the server which events to send, opening the socket the first time. */
  private say(): void {
    if (!this.socket) {
      const socket = this.open()
      this.socket = socket
      socket.onmessage = (message) => this.heard(String(message.data))
      socket.onopen = () => this.say()
      /* Gone: reopened, and told again, in a moment. */
      socket.onclose = () => {
        if (this.socket !== socket) return
        this.socket = null
        setTimeout(() => {
          if (!this.socket && this.listeners.size > 0) this.say()
        }, 1000)
      }
      return
    }
    if (this.socket.readyState === WebSocket.OPEN) this.socket.send(JSON.stringify({ listen: [...this.listeners.keys()] }))
  }
}

/** The stand-in for Tauri's internals, over `/rpc` and `/events`. */
export function browserInternals(events: Events, send: typeof fetch = fetch): Internals {
  const callbacks = new Map<number, Callback>()
  let nextCallback = 1
  return {
    invoke: async (command, args) => {
      if (command === 'plugin:event|listen') {
        const { event, handler } = args as { event: string; handler: number }
        const call = callbacks.get(handler)
        return events.listen(event, (payload) => call?.(payload))
      }
      if (command === 'plugin:event|unlisten') {
        const { event, eventId } = args as { event: string; eventId: number }
        events.unlisten(event, eventId)
        return null
      }
      /* A plugin's command is the desktop's (a dialog, the clipboard): not here. */
      if (command.startsWith('plugin:')) throw { message: 'only in the desktop app', status: 501 }
      return rpc(command, args, send)
    },
    transformCallback: (callback, once = false) => {
      const id = nextCallback++
      callbacks.set(id, (payload) => {
        if (once) callbacks.delete(id)
        callback(payload)
      })
      return id
    },
    unregisterCallback: (id) => void callbacks.delete(id),
    convertFileSrc: (path) => path,
    metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
  }
}

/**
 * Puts the stand-in in place when the page is in a browser. Called once,
 * before the app renders; in the desktop window it does nothing.
 */
export function installBrowserTransport(): void {
  if (typeof window === 'undefined' || window.__TAURI_INTERNALS__) return
  if (!/^https?:$/.test(window.location.protocol)) return
  const host = window.location.hostname
  if (host !== '127.0.0.1' && host !== 'localhost') return
  window.__DEVPIT_BROWSER__ = true
  const events = new Events(() => new WebSocket(`ws://${window.location.host}/events`))
  window.__TAURI_INTERNALS__ = browserInternals(events)
  /* `unlisten` drops the listener here before it asks the backend to. */
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: (event, id) => events.unlisten(event, id) }
}
