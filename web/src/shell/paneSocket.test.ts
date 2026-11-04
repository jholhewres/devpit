import { describe, expect, it } from 'vitest'

import { Kind, PaneSocket, frame } from './paneSocket'

/* A socket opened by hand: what the page sent, and frames pushed back. */
class FakeSocket {
  binaryType = 'blob'
  readyState = 0
  sent: Uint8Array[] = []
  onopen: (() => void) | null = null
  onclose: (() => void) | null = null
  onerror: (() => void) | null = null
  onmessage: ((event: { data: ArrayBuffer }) => void) | null = null
  send(data: ArrayBuffer | Uint8Array) {
    this.sent.push(new Uint8Array(data instanceof Uint8Array ? data : data))
  }
  close() {
    this.readyState = 3
    this.onclose?.()
  }
  opened() {
    this.readyState = 1
    this.onopen?.()
  }
  hear(handle: number, kind: number, bytes: number[] | Uint8Array) {
    this.onmessage?.({ data: frame(handle, kind, Uint8Array.from(bytes)).buffer as ArrayBuffer })
  }
}

const decode = (wire: Uint8Array) => ({
  handle: new DataView(wire.buffer, wire.byteOffset).getUint32(0, true),
  kind: wire[4],
  bytes: wire.slice(5),
})
const text = (bytes: Uint8Array) => new TextDecoder().decode(bytes)
const settle = () => new Promise((done) => setTimeout(done, 0))

function setup() {
  const sockets: FakeSocket[] = []
  const panes = new PaneSocket(async () => {
    const socket = new FakeSocket()
    sockets.push(socket)
    return socket as unknown as WebSocket
  })
  return { panes, sockets }
}

function listener() {
  const heard: { bytes: number[]; replay: boolean }[] = []
  const ended: (string | null)[] = []
  return {
    heard,
    ended,
    on: {
      output: (bytes: Uint8Array, replay: boolean) => heard.push({ bytes: [...bytes], replay }),
      end: (reason: string | null) => ended.push(reason),
    },
  }
}

describe('panes over one binary socket', () => {
  it('frames are the pane handle, the kind and the bytes', () => {
    const wire = frame(258, Kind.Input, Uint8Array.from([7, 8]))
    expect([...wire]).toEqual([2, 1, 0, 0, Kind.Input, 7, 8])
  })

  it('asks for the pane once the socket opens, then carries its bytes both ways', async () => {
    const { panes, sockets } = setup()
    const l = listener()
    const pane = panes.attach('prj_1', 'leaf_1', { rows: 24, cols: 80 }, l.on)
    pane.write('ls\r')
    await settle()
    expect(sockets).toHaveLength(1)
    expect(sockets[0].sent).toEqual([])
    sockets[0].opened()
    const [asked, typed] = sockets[0].sent.map(decode)
    expect(asked.kind).toBe(Kind.Attach)
    expect(JSON.parse(text(asked.bytes))).toEqual({ projectId: 'prj_1', paneId: 'leaf_1', rows: 24, cols: 80 })
    expect({ ...typed, bytes: [...typed.bytes] }).toEqual({ handle: asked.handle, kind: Kind.Input, bytes: [108, 115, 13] })

    sockets[0].hear(asked.handle, Kind.Replay, [1])
    sockets[0].hear(asked.handle, Kind.Output, [2, 3])
    sockets[0].hear(asked.handle + 1, Kind.Output, [9])
    expect(l.heard).toEqual([
      { bytes: [1], replay: true },
      { bytes: [2, 3], replay: false },
    ])
  })

  it('gives each attach its own handle on the one socket', async () => {
    const { panes, sockets } = setup()
    panes.attach('p', 'a', { rows: 1, cols: 1 }, listener().on)
    panes.attach('p', 'b', { rows: 1, cols: 1 }, listener().on)
    await settle()
    sockets[0].opened()
    const handles = sockets[0].sent.map(decode).map((one) => one.handle)
    expect(sockets).toHaveLength(1)
    expect(new Set(handles).size).toBe(2)
  })

  it('acknowledges, resizes and detaches by handle', async () => {
    const { panes, sockets } = setup()
    const pane = panes.attach('p', 'a', { rows: 1, cols: 1 }, listener().on)
    await settle()
    sockets[0].opened()
    const handle = decode(sockets[0].sent[0]).handle
    pane.ack(65536)
    const sized = pane.resize(30, 100)
    const last = () => decode(sockets[0].sent.at(-1)!)
    expect(decode(sockets[0].sent[1])).toEqual({ handle, kind: Kind.Ack, bytes: Uint8Array.from([0, 0, 1, 0]) })
    expect(last()).toEqual({ handle, kind: Kind.Resize, bytes: Uint8Array.from([30, 0, 100, 0]) })
    sockets[0].hear(handle, Kind.Resize, [30, 0, 90, 0])
    await expect(sized).resolves.toEqual({ rows: 30, cols: 90 })
    pane.detach()
    expect(last()).toEqual({ handle, kind: Kind.Detach, bytes: new Uint8Array() })
  })

  it('refuses a resize the server could not apply', async () => {
    const { panes, sockets } = setup()
    const pane = panes.attach('p', 'a', { rows: 1, cols: 1 }, listener().on)
    await settle()
    sockets[0].opened()
    const handle = decode(sockets[0].sent[0]).handle
    const sized = pane.resize(30, 100)
    sockets[0].hear(handle, Kind.Resize, [])
    await expect(sized).rejects.toBeDefined()
  })

  it('pages older history by handle', async () => {
    const { panes, sockets } = setup()
    const pane = panes.attach('p', 'a', { rows: 1, cols: 1 }, listener().on)
    await settle()
    sockets[0].opened()
    const handle = decode(sockets[0].sent[0]).handle
    const older = pane.older()
    expect(decode(sockets[0].sent.at(-1)!)).toEqual({ handle, kind: Kind.Older, bytes: new Uint8Array() })
    sockets[0].hear(handle, Kind.Older, [5, 0, 0, 0, 0, 0, 0, 0, 65, 66])
    await expect(older).resolves.toEqual({ left: 5, bytes: Uint8Array.from([65, 66]) })
  })

  it('says why a pane ended, and an empty end is a plain exit', async () => {
    const { panes, sockets } = setup()
    const a = listener()
    const b = listener()
    panes.attach('p', 'a', { rows: 1, cols: 1 }, a.on)
    panes.attach('p', 'b', { rows: 1, cols: 1 }, b.on)
    await settle()
    sockets[0].opened()
    const [ha, hb] = sockets[0].sent.map(decode).map((one) => one.handle)
    sockets[0].hear(ha, Kind.End, new TextEncoder().encode('no such pane'))
    sockets[0].hear(hb, Kind.End, [])
    expect(a.ended).toEqual(['no such pane'])
    expect(b.ended).toEqual([null])
  })

  it('ends every pane when the socket goes, and opens a new one for the next attach', async () => {
    const { panes, sockets } = setup()
    const a = listener()
    panes.attach('p', 'a', { rows: 1, cols: 1 }, a.on)
    await settle()
    sockets[0].opened()
    sockets[0].close()
    expect(a.ended).toHaveLength(1)
    expect(a.ended[0]).toMatch(/connection/)
    panes.attach('p', 'b', { rows: 1, cols: 1 }, listener().on)
    await settle()
    expect(sockets).toHaveLength(2)
  })

  it('ends a pane when no socket can be opened', async () => {
    const panes = new PaneSocket(async () => null)
    const a = listener()
    panes.attach('p', 'a', { rows: 1, cols: 1 }, a.on)
    await settle()
    expect(a.ended).toHaveLength(1)
  })
})
