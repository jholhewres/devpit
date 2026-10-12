import { afterEach, describe, expect, it, vi } from 'vitest'

import type { RemoteOut } from '../gen/bindings'
import { Handshake, hex, keyPair, type Session } from './noise'
import { CLOSED, connectRelay, pairingIn, relayUrl, type RelayMachine } from './relay'

/* A socket the test holds the other end of: what the page sends lands in
   `sent`, and the test answers as the relay and the machine would. */
class Wire {
  static OPEN = 1
  binaryType = 'blob'
  readyState = 0
  sent: Uint8Array[] = []
  onopen: (() => void) | null = null
  onmessage: ((event: { data: unknown }) => void) | null = null
  onclose: ((event: { code: number }) => void) | null = null
  closed = false
  readonly url: string
  constructor(url: string) {
    this.url = url
  }
  send(data: Uint8Array): void {
    this.sent.push(new Uint8Array(data))
  }
  close(): void {
    this.closed = true
  }
  open(): void {
    this.readyState = 1
    this.onopen?.()
  }
  deliver(bytes: Uint8Array): void {
    this.onmessage?.({ data: bytes.slice().buffer })
  }
  shut(code: number): void {
    this.readyState = 3
    this.onclose?.({ code })
  }
}

const HOST = 'host_01RELAYTEST'
const words = new TextEncoder()
const reading = new TextDecoder()

function setUp(machineKeys = keyPair(), named = machineKeys) {
  const sockets: Wire[] = []
  const heard: RemoteOut[] = []
  const states: [boolean, string | undefined, number | undefined][] = []
  const machine: RelayMachine = { host: HOST, key: hex(named.public), name: 'notebook' }
  const device = keyPair()
  const link = connectRelay(machine, device, (said) => heard.push(said), (up, why, code) => states.push([up, why, code]), {
    origin: 'https://devpit.test',
    pairing: { secret: 'ab'.repeat(32), name: 'Pixel' },
    open: (url) => {
      const socket = new Wire(url)
      sockets.push(socket)
      return socket as unknown as WebSocket
    },
  })
  return { sockets, heard, states, link, device, machineKeys }
}

/* The machine's half of a handshake, against what the page sent. */
function answer(wire: Wire, keys: ReturnType<typeof keyPair>): { session: () => Session; handshake: Handshake } {
  const handshake = new Handshake(false, keys, HOST)
  handshake.read(wire.sent[0]!)
  wire.deliver(handshake.write())
  return {
    handshake,
    session: () => {
      handshake.read(wire.sent[1]!)
      return handshake.session(1 << 20)
    },
  }
}

afterEach(() => vi.useRealTimers())

describe('the viewer through the relay', () => {
  it('asks the relay on its own origin, for the machine its code named', () => {
    expect(relayUrl('https://devpit.app', 'host_1')).toBe('wss://devpit.app/v1/relay/view?host=host_1')
    expect(relayUrl('http://127.0.0.1:8787', 'host_1')).toBe('ws://127.0.0.1:8787/v1/relay/view?host=host_1')
  })

  it('reads a pairing link, and nothing that is not one', () => {
    const key = '1f'.repeat(32)
    const secret = 'a0'.repeat(32)
    expect(pairingIn(`#relay=host_1&key=${key}&secret=${secret}&name=box`)).toEqual({ machine: { host: 'host_1', key, name: 'box' }, secret })
    expect(pairingIn(`#relay=host_1&key=${key}`)).toBeNull()
    expect(pairingIn(`#relay=../x&key=${key}&secret=${secret}`)).toBeNull()
    expect(pairingIn(`#relay=host_1&key=zz&secret=${secret}`)).toBeNull()
    expect(pairingIn('#pair=ABCD2345')).toBeNull()
  })

  it('pairs once the machine proves its key, then hears it sealed', () => {
    const { sockets, heard, states, machineKeys, device } = setUp()
    const wire = sockets[0]!
    expect(wire.url).toBe(`wss://devpit.test/v1/relay/view?host=${HOST}`)
    wire.open()
    const machine = answer(wire, machineKeys)
    const session = machine.session()
    expect(hex(session.peer)).toBe(hex(device.public))
    expect(states.at(-1)?.[0]).toBe(true)

    // The first thing said in the session is the pairing, sealed.
    const first = JSON.parse(reading.decode(session.open(wire.sent[2]!)!)) as { t: string; secret: string; name: string }
    expect(first).toEqual({ t: 'pair', secret: 'ab'.repeat(32), name: 'Pixel' })
    expect(wire.sent.some((frame) => reading.decode(frame).includes('ab'.repeat(32)))).toBe(false)

    const welcome = { t: 'welcome', device: 'Pixel', host: 'notebook', typing: false, answering: false }
    for (const frame of session.seal(words.encode(JSON.stringify(welcome)))) wire.deliver(frame)
    expect(heard).toEqual([welcome])
  })

  it('says nothing of itself to a machine that is not the one it paired with', () => {
    const { sockets, states } = setUp(keyPair(), keyPair())
    const wire = sockets[0]!
    wire.open()
    const impostor = new Handshake(false, keyPair(), HOST)
    impostor.read(wire.sent[0]!)
    wire.deliver(impostor.write())
    expect(wire.sent).toHaveLength(1)
    expect(wire.closed).toBe(true)
    expect(states.at(-1)?.[1]).toMatch(/not the machine/)
  })

  it('tells the person to sign in, and waits for them rather than asking again', () => {
    vi.useFakeTimers()
    const { sockets, states } = setUp()
    sockets[0]!.shut(CLOSED.signIn)
    expect(states.at(-1)).toEqual([false, expect.stringMatching(/Sign in/), CLOSED.signIn])
    vi.advanceTimersByTime(60_000)
    expect(sockets).toHaveLength(1)
  })

  it('connects again, later, when the machine is away', () => {
    vi.useFakeTimers()
    const { sockets, states } = setUp()
    sockets[0]!.shut(CLOSED.offline)
    expect(states.at(-1)?.[1]).toMatch(/not connected/)
    vi.advanceTimersByTime(20_000)
    expect(sockets).toHaveLength(2)
  })

  it('ends the session when a frame does not open', () => {
    const { sockets, machineKeys } = setUp()
    const wire = sockets[0]!
    wire.open()
    const session = answer(wire, machineKeys).session()
    const frame = session.seal(words.encode('{"t":"pong"}'))[0]!
    frame[1]! ^= 1
    wire.deliver(frame)
    expect(wire.closed).toBe(true)
  })
})
