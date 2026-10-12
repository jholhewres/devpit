import type { RemoteIn, RemoteOut } from '../gen/bindings'
import { Handshake, hex, keyPair, type KeyPair, NoiseError, same, type Session, unhex } from './noise'
import { backoff, type Link } from './socket'

/*
 * The viewer through devpit.app's relay. The page opens a WebSocket to the
 * relay, which joins it to the machine; inside it, a Noise session with the
 * machine whose key the QR code carried. devpit.app passes the frames along
 * and can open none of them.
 *
 * Pairing is the QR code: the machine's id, its key and a one-time secret, in
 * the link's fragment, which a browser never sends anywhere. The key is what
 * the handshake checks the machine against; the secret goes to the machine
 * once, inside the session, and only after the machine proved that key.
 */

/** A machine reached through the relay, as its QR code named it. */
export interface RelayMachine {
  host: string
  /** Its static X25519 key, hex. */
  key: string
  name: string
}

/** What a device sends the machine it has not paired with yet, once. */
export interface RelayPairing {
  secret: string
  name: string
}

/** What the relay closes with, when a page can do something about it. */
export const CLOSED = { signIn: 4401, offline: 4404, tooMany: 4429, late: 4504 } as const

/** The most a message from the machine may be: a big board, a long chat. */
const MOST_MESSAGE = 8 * 1024 * 1024

const KEPT = 'devpit.remote.relay'

interface Kept {
  /** This device's static key, hex. */
  secret: string
  machines: RelayMachine[]
}

function kept(): Kept | null {
  try {
    const said = JSON.parse(localStorage.getItem(KEPT) ?? 'null') as Kept | null
    return said && typeof said.secret === 'string' && Array.isArray(said.machines) ? said : null
  } catch {
    return null
  }
}

function keep(next: Kept): void {
  try {
    localStorage.setItem(KEPT, JSON.stringify(next))
  } catch {
    // Unkept, the device pairs again next time.
  }
}

/** This device's key, made the first time it is asked for. */
export function deviceKey(): KeyPair {
  const secret = unhex(kept()?.secret ?? '', 32)
  if (secret) return keyPair(secret)
  const made = keyPair()
  keep({ secret: hex(made.secret), machines: kept()?.machines ?? [] })
  return made
}

export function machinesKept(): RelayMachine[] {
  return kept()?.machines ?? []
}

export function keepMachine(machine: RelayMachine): void {
  const key = deviceKey()
  const others = machinesKept().filter((one) => one.host !== machine.host)
  keep({ secret: hex(key.secret), machines: [machine, ...others] })
}

export function forgetMachine(host: string): void {
  const now = kept()
  if (now) keep({ ...now, machines: now.machines.filter((one) => one.host !== host) })
}

/** What a pairing link carries in its fragment, when it is one. */
export function pairingIn(fragment: string): { machine: RelayMachine; secret: string } | null {
  const said = new URLSearchParams(fragment.replace(/^#/, ''))
  const host = said.get('relay') ?? ''
  const key = said.get('key') ?? ''
  const secret = said.get('secret') ?? ''
  if (!/^[A-Za-z0-9_-]{1,64}$/.test(host) || !unhex(key, 32) || !unhex(secret, 32)) return null
  return { machine: { host, key: key.toLowerCase(), name: (said.get('name') ?? '').slice(0, 80) || 'your machine' }, secret }
}

const OFFER = 'devpit.remote.offer'

/** A pairing link's fragment, kept for the page that comes back from signing in. */
export function holdOffer(fragment: string): void {
  try {
    sessionStorage.setItem(OFFER, fragment)
  } catch {
    // Not kept: the code is scanned again after signing in.
  }
}

/** The pairing this page was opened with, or came back to: taken once. */
export function offerOpened(fragment: string): { machine: RelayMachine; secret: string } | null {
  let held = ''
  try {
    held = sessionStorage.getItem(OFFER) ?? ''
    sessionStorage.removeItem(OFFER)
  } catch {
    // Nothing held.
  }
  return pairingIn(fragment) ?? pairingIn(held)
}

/** The relay's address for `host`, on this page's own origin. */
export function relayUrl(origin: string, host: string): string {
  return `${origin.replace(/^http/, 'ws')}/v1/relay/view?host=${encodeURIComponent(host)}`
}

/** Why the relay closed, in words a person can act on. */
export function closedBecause(code: number): string | null {
  switch (code) {
    case CLOSED.signIn:
      return 'Sign in to devpit.app on this device, with the account the machine is signed in to.'
    case CLOSED.offline:
      return 'The machine is not connected to devpit.app: it is off, asleep, or Remote is off there.'
    case CLOSED.tooMany:
      return 'Too many devices are connected to this machine right now.'
    case CLOSED.late:
      return 'The machine did not answer in time.'
    default:
      return null
  }
}

export type Opener = (url: string) => WebSocket

export interface RelayOptions {
  /** The relay's origin: the page's own, where devpit.app serves it. */
  origin: string
  /** Sent once the machine proves its key, when the device is not paired yet. */
  pairing?: RelayPairing | null
  open?: Opener
}

const words = new TextEncoder()
const reading = new TextDecoder()

/** A frame as a socket sends it: bytes of its own, on a plain ArrayBuffer. */
const outgoing = (frame: Uint8Array): Uint8Array<ArrayBuffer> => new Uint8Array(frame)

/**
 * Connects to `machine` through the relay, as `device`. Same `Link` as the
 * tailnet's, so the viewer cannot tell which way it came.
 */
export function connectRelay(
  machine: RelayMachine,
  device: KeyPair,
  heard: (message: RemoteOut) => void,
  state: (up: boolean, why?: string, code?: number) => void,
  options: RelayOptions,
): Link {
  const open = options.open ?? ((url: string) => new WebSocket(url))
  const expected = unhex(machine.key, 32)
  let socket: WebSocket | null = null
  let session: Session | null = null
  let tries = 0
  let closed = false
  let again: ReturnType<typeof setTimeout> | undefined
  let beat: ReturnType<typeof setInterval> | undefined
  let owed = options.pairing ?? null

  const send = (message: RemoteIn): void => {
    if (!session || socket?.readyState !== WebSocket.OPEN) return
    for (const frame of session.seal(words.encode(JSON.stringify(message)))) socket.send(outgoing(frame))
  }

  const end = (why: string): void => {
    closed = true
    state(false, why)
    socket?.close()
  }

  const connectOnce = (): void => {
    if (!expected) return end('This pairing link is not one devpit made.')
    const handshake = new Handshake(true, device, machine.host)
    session = null
    const here = open(relayUrl(options.origin, machine.host))
    socket = here
    here.binaryType = 'arraybuffer'
    here.onopen = () => here.send(outgoing(handshake.write()))
    here.onmessage = (event) => {
      if (!(event.data instanceof ArrayBuffer)) return here.close()
      const frame = new Uint8Array(event.data)
      try {
        if (!session) {
          handshake.read(frame)
          // Before this device says who it is, the machine must have.
          if (!handshake.peer || !same(handshake.peer, expected)) return end('This is not the machine this device paired with. Pair it again.')
          here.send(outgoing(handshake.write()))
          session = handshake.session(MOST_MESSAGE)
          tries = 0
          state(true)
          if (owed) {
            send({ t: 'pair', secret: owed.secret, name: owed.name })
            owed = null
          }
          beat = setInterval(() => send({ t: 'ping' }), 30_000)
          return
        }
        const whole = session.open(frame)
        if (whole) heard(JSON.parse(reading.decode(whole)) as RemoteOut)
      } catch (err) {
        // A frame that does not open ends the session; the next one starts over.
        if (err instanceof NoiseError || err instanceof SyntaxError) here.close()
        else throw err
      }
    }
    here.onclose = (event) => {
      clearInterval(beat)
      session = null
      if (closed) return
      const why = closedBecause(event.code)
      state(false, why ?? undefined, event.code)
      if (event.code === CLOSED.signIn) return
      again = setTimeout(connectOnce, backoff(tries++))
    }
  }
  connectOnce()

  return {
    send,
    close: () => {
      closed = true
      clearTimeout(again)
      clearInterval(beat)
      session = null
      socket?.close()
    },
  }
}
