/*
 * Every terminal's bytes on one binary WebSocket, `/panes` on the loopback
 * server: in the desktop window (which cheaper than Tauri's channel, an
 * `eval` and a `fetch` per frame) and in a browser alike.
 *
 * A frame is `[u32 handle][u8 kind][bytes]`, little-endian, the handle being
 * this page's number for one attach. `apps/desktop/src/pane_socket.rs` is the
 * other end and keeps the same kinds.
 */

export const Kind = {
  /** Server → page: what the pane printed. */
  Output: 0,
  /** Page → server: bytes parsed (u32), so the pty reads on. */
  Ack: 1,
  /** Page → server: typing. */
  Input: 2,
  /** Page → server: `rows u16, cols u16`; server → page: the size applied, or nothing when refused. */
  Resize: 3,
  /** Page → server: `{ projectId, paneId, rows, cols }` as JSON. */
  Attach: 4,
  /** Page → server: let the pane go. */
  Detach: 5,
  /** Server → page: the pane ended; the reason as text, nothing for a plain exit. */
  End: 6,
  /** Server → page: what the pane printed before this attach (not acknowledged). */
  Replay: 7,
  /** Page → server: the page before what it holds; server → page: `left u64`, then the bytes. */
  Older: 8,
} as const

export function frame(handle: number, kind: number, bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  const wire = new Uint8Array(5 + bytes.length)
  new DataView(wire.buffer).setUint32(0, handle, true)
  wire[4] = kind
  wire.set(bytes, 5)
  return wire
}

export interface PaneListener {
  output: (bytes: Uint8Array, replay: boolean) => void
  /** Why it ended; `null` for a pane whose program exited. */
  end: (reason: string | null) => void
}

export interface PaneStream {
  write: (data: string) => void
  resize: (rows: number, cols: number) => Promise<{ rows: number; cols: number } | null>
  /** The page of history before what this attach holds; `null` when there is none to ask. */
  older: () => Promise<{ left: number; bytes: Uint8Array } | null>
  ack: (bytes: number) => void
  detach: () => void
}

interface Open {
  on: PaneListener
  sizing: { resolve: (size: { rows: number; cols: number }) => void; reject: (why: Error) => void }[]
  paging: ((page: { left: number; bytes: Uint8Array } | null) => void)[]
}

/** The end every pane gets when the socket closes under it: worth attaching again. */
export const LOST = 'the terminal connection closed'
/** The end every pane gets when the socket could not be opened at all. */
export const UNAVAILABLE = 'the terminal connection could not be opened'
const encoder = new TextEncoder()
const decoder = new TextDecoder()

export class PaneSocket {
  private readonly open: () => Promise<WebSocket | null>
  private socket: WebSocket | null = null
  private opening: Promise<WebSocket | null> | null = null
  private waiting: Uint8Array<ArrayBuffer>[] = []
  private readonly panes = new Map<number, Open>()
  private next = 1

  constructor(open: () => Promise<WebSocket | null>) {
    this.open = open
  }

  attach(projectId: string, paneId: string, size: { rows: number; cols: number }, on: PaneListener): PaneStream {
    const handle = this.next++
    const open: Open = { on, sizing: [], paging: [] }
    this.panes.set(handle, open)
    this.send(handle, Kind.Attach, encoder.encode(JSON.stringify({ projectId, paneId, rows: size.rows, cols: size.cols })))
    const u16s = (a: number, b: number): Uint8Array => {
      const bytes = new Uint8Array(4)
      const view = new DataView(bytes.buffer)
      view.setUint16(0, a, true)
      view.setUint16(2, b, true)
      return bytes
    }
    return {
      write: (data) => this.send(handle, Kind.Input, encoder.encode(data)),
      resize: (rows, cols) =>
        new Promise((resolve, reject) => {
          if (!this.panes.has(handle)) return resolve(null)
          open.sizing.push({ resolve, reject })
          this.send(handle, Kind.Resize, u16s(rows, cols))
        }),
      older: () =>
        new Promise((resolve) => {
          if (!this.panes.has(handle)) return resolve(null)
          open.paging.push(resolve)
          this.send(handle, Kind.Older, new Uint8Array())
        }),
      ack: (bytes) => {
        const wire = new Uint8Array(4)
        new DataView(wire.buffer).setUint32(0, bytes, true)
        this.send(handle, Kind.Ack, wire)
      },
      detach: () => {
        if (!this.panes.delete(handle)) return
        settle(open)
        this.send(handle, Kind.Detach, new Uint8Array())
      },
    }
  }

  private send(handle: number, kind: number, bytes: Uint8Array): void {
    const wire = frame(handle, kind, bytes)
    if (this.socket && this.socket.readyState === 1) return this.socket.send(wire)
    this.waiting.push(wire)
    this.connect()
  }

  private connect(): void {
    if (this.opening || this.socket) return
    this.opening = this.open().catch(() => null)
    void this.opening.then((socket) => {
      if (!socket) {
        this.opening = null
        this.waiting = []
        this.endAll(UNAVAILABLE)
        return
      }
      socket.binaryType = 'arraybuffer'
      socket.onopen = () => {
        this.socket = socket
        this.opening = null
        const waiting = this.waiting
        this.waiting = []
        for (const wire of waiting) socket.send(wire)
      }
      socket.onmessage = (event: MessageEvent) => this.heard(new Uint8Array(event.data as ArrayBuffer))
      socket.onclose = () => {
        const opened = this.socket === socket
        if (!opened && this.opening === null) return
        this.socket = null
        this.opening = null
        this.waiting = []
        this.endAll(opened ? LOST : UNAVAILABLE)
      }
    })
  }

  private endAll(reason: string): void {
    const all = [...this.panes.values()]
    this.panes.clear()
    for (const open of all) {
      settle(open)
      open.on.end(reason)
    }
  }

  private heard(wire: Uint8Array): void {
    if (wire.length < 5) return
    const handle = new DataView(wire.buffer, wire.byteOffset).getUint32(0, true)
    const open = this.panes.get(handle)
    if (!open) return
    const bytes = wire.subarray(5)
    switch (wire[4]) {
      case Kind.Output:
        return open.on.output(bytes, false)
      case Kind.Replay:
        return open.on.output(bytes, true)
      case Kind.Resize: {
        const asked = open.sizing.shift()
        if (!asked) return
        if (bytes.length < 4) return asked.reject(new Error('the pane could not be resized'))
        const view = new DataView(bytes.buffer, bytes.byteOffset)
        return asked.resolve({ rows: view.getUint16(0, true), cols: view.getUint16(2, true) })
      }
      case Kind.Older: {
        const asked = open.paging.shift()
        if (!asked) return
        if (bytes.length < 8) return asked(null)
        const left = Number(new DataView(bytes.buffer, bytes.byteOffset).getBigUint64(0, true))
        return asked({ left, bytes: bytes.slice(8) })
      }
      case Kind.End:
        this.panes.delete(handle)
        settle(open)
        return open.on.end(bytes.length > 0 ? decoder.decode(bytes) : null)
    }
  }
}

/** Nothing waits on a pane that is gone. */
function settle(open: Open): void {
  for (const asked of open.sizing.splice(0)) asked.reject(new Error(LOST))
  for (const asked of open.paging.splice(0)) asked(null)
}
