import { chacha20poly1305 } from '@noble/ciphers/chacha.js'
import { x25519 } from '@noble/curves/ed25519.js'
import { blake2s } from '@noble/hashes/blake2.js'
import { hmac } from '@noble/hashes/hmac.js'

/*
 * Noise, as the viewer speaks it through the relay: Noise_XX_25519_ChaChaPoly_BLAKE2s,
 * the device beginning. The primitives are @noble's, audited; what is here
 * is the protocol's own bookkeeping, spec revision 34, held byte for byte to
 * a transcript snow made (`crates/remote/tests/noise-xx.json`).
 *
 * A message travels as frames, as on the machine: each frame's plaintext is a
 * flag (0 more, 1 last) and a chunk, and its nonce is its place in the
 * session — so a frame dropped, repeated or moved does not open.
 */

export const PATTERN = 'Noise_XX_25519_ChaChaPoly_BLAKE2s'
export const MOST_FRAME = 65535
const TAG = 16
export const MOST_CHUNK = MOST_FRAME - TAG - 1
/** As on the machine: far past a real session, and exact in a double. */
export const MOST_FRAMES = 2 ** 48
const HASHLEN = 32
const MORE = 0
const LAST = 1

export type NoiseFault = 'broken' | 'too-big' | 'worn' | 'out-of-turn'

export class NoiseError extends Error {
  readonly fault: NoiseFault
  constructor(fault: NoiseFault) {
    super(`noise: ${fault}`)
    this.fault = fault
  }
}

export interface KeyPair {
  secret: Uint8Array
  public: Uint8Array
}

export function keyPair(secret: Uint8Array = x25519.utils.randomSecretKey()): KeyPair {
  return { secret, public: x25519.getPublicKey(secret) }
}

const text = new TextEncoder()

export function prologue(host: string): Uint8Array {
  return text.encode(`devpit-remote/1 ${host}`)
}

function concat(...parts: readonly Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0))
  let at = 0
  for (const part of parts) {
    out.set(part, at)
    at += part.length
  }
  return out
}

export function same(one: Uint8Array, other: Uint8Array): boolean {
  if (one.length !== other.length) return false
  let differs = 0
  for (let at = 0; at < one.length; at++) differs |= one[at]! ^ other[at]!
  return differs === 0
}

/** Four zero bytes and the counter, 64-bit little-endian. */
function nonce(n: number): Uint8Array {
  const out = new Uint8Array(12)
  const view = new DataView(out.buffer)
  view.setUint32(4, n >>> 0, true)
  view.setUint32(8, Math.floor(n / 2 ** 32), true)
  return out
}

function hkdf(chainingKey: Uint8Array, input: Uint8Array): [Uint8Array, Uint8Array] {
  const temp = hmac(blake2s, chainingKey, input)
  const first = hmac(blake2s, temp, Uint8Array.of(1))
  const second = hmac(blake2s, temp, concat(first, Uint8Array.of(2)))
  return [first, second]
}

class CipherState {
  private key: Uint8Array | null = null
  private n = 0

  constructor(key: Uint8Array | null = null) {
    this.key = key
  }

  get keyed(): boolean {
    return this.key !== null
  }

  rekey(key: Uint8Array): void {
    this.key = key
    this.n = 0
  }

  encrypt(ad: Uint8Array, plain: Uint8Array): Uint8Array {
    if (!this.key) return plain
    if (this.n >= MOST_FRAMES) throw new NoiseError('worn')
    const sealed = chacha20poly1305(this.key, nonce(this.n), ad).encrypt(plain)
    this.n++
    return sealed
  }

  decrypt(ad: Uint8Array, sealed: Uint8Array): Uint8Array {
    if (!this.key) return sealed
    if (this.n >= MOST_FRAMES) throw new NoiseError('worn')
    let plain: Uint8Array
    try {
      plain = chacha20poly1305(this.key, nonce(this.n), ad).decrypt(sealed)
    } catch {
      throw new NoiseError('broken')
    }
    this.n++
    return plain
  }
}

class SymmetricState {
  chainingKey: Uint8Array
  hash: Uint8Array
  readonly cipher = new CipherState()

  constructor(name: string) {
    const bytes = text.encode(name)
    this.hash = bytes.length <= HASHLEN ? concat(bytes, new Uint8Array(HASHLEN - bytes.length)) : blake2s(bytes)
    this.chainingKey = this.hash.slice()
  }

  mixKey(input: Uint8Array): void {
    const [chainingKey, key] = hkdf(this.chainingKey, input)
    this.chainingKey = chainingKey
    this.cipher.rekey(key)
  }

  mixHash(data: Uint8Array): void {
    this.hash = blake2s(concat(this.hash, data))
  }

  encryptAndHash(plain: Uint8Array): Uint8Array {
    const sealed = this.cipher.encrypt(this.hash, plain)
    this.mixHash(sealed)
    return sealed
  }

  decryptAndHash(sealed: Uint8Array): Uint8Array {
    const plain = this.cipher.decrypt(this.hash, sealed)
    this.mixHash(sealed)
    return plain
  }

  split(): [CipherState, CipherState] {
    const [first, second] = hkdf(this.chainingKey, new Uint8Array(0))
    return [new CipherState(first), new CipherState(second)]
  }
}

function dh(secret: Uint8Array, other: Uint8Array): Uint8Array {
  try {
    return x25519.getSharedSecret(secret, other)
  } catch {
    // A low-order key: nobody's honest key.
    throw new NoiseError('broken')
  }
}

/** XX: → e · ← e, ee, s, es · → s, se. Payloads are empty. */
export class Handshake {
  private readonly initiator: boolean
  private readonly local: KeyPair
  private ephemeral: KeyPair | null
  private readonly state: SymmetricState
  private remoteEphemeral: Uint8Array | null = null
  private remoteStatic: Uint8Array | null = null
  private step = 0

  /** `ephemeral` is for a test vector alone. */
  constructor(initiator: boolean, local: KeyPair, host: string, ephemeral: KeyPair | null = null) {
    this.initiator = initiator
    this.local = local
    this.ephemeral = ephemeral
    this.state = new SymmetricState(PATTERN)
    this.state.mixHash(prologue(host))
  }

  get writesNext(): boolean {
    return !this.finished && this.step % 2 === (this.initiator ? 0 : 1)
  }

  get finished(): boolean {
    return this.step === 3
  }

  /** The other end's static key, once it has said it. */
  get peer(): Uint8Array | null {
    return this.remoteStatic
  }

  write(): Uint8Array {
    if (!this.writesNext) throw new NoiseError('out-of-turn')
    const out: Uint8Array[] = []
    if (this.step === 0) {
      this.ephemeral ??= keyPair()
      out.push(this.ephemeral.public)
      this.state.mixHash(this.ephemeral.public)
    } else if (this.step === 1) {
      this.ephemeral ??= keyPair()
      out.push(this.ephemeral.public)
      this.state.mixHash(this.ephemeral.public)
      this.state.mixKey(dh(this.ephemeral.secret, this.remoteEphemeral!))
      out.push(this.state.encryptAndHash(this.local.public))
      this.state.mixKey(dh(this.local.secret, this.remoteEphemeral!))
    } else {
      out.push(this.state.encryptAndHash(this.local.public))
      this.state.mixKey(dh(this.local.secret, this.remoteEphemeral!))
    }
    out.push(this.state.encryptAndHash(new Uint8Array(0)))
    this.step++
    return concat(...out)
  }

  read(message: Uint8Array): void {
    if (this.writesNext || this.finished) throw new NoiseError('out-of-turn')
    if (message.length > MOST_FRAME) throw new NoiseError('too-big')
    let at = 0
    const take = (count: number): Uint8Array => {
      if (at + count > message.length) throw new NoiseError('broken')
      const part = message.subarray(at, at + count)
      at += count
      return part
    }
    if (this.step === 0) {
      this.remoteEphemeral = take(32).slice()
      this.state.mixHash(this.remoteEphemeral)
    } else if (this.step === 1) {
      this.remoteEphemeral = take(32).slice()
      this.state.mixHash(this.remoteEphemeral)
      this.state.mixKey(dh(this.ephemeral!.secret, this.remoteEphemeral))
      this.remoteStatic = this.state.decryptAndHash(take(32 + TAG)).slice()
      this.state.mixKey(dh(this.ephemeral!.secret, this.remoteStatic))
    } else {
      this.remoteStatic = this.state.decryptAndHash(take(32 + TAG)).slice()
      this.state.mixKey(dh(this.ephemeral!.secret, this.remoteStatic))
    }
    const payload = this.state.decryptAndHash(message.subarray(at))
    if (payload.length !== 0) throw new NoiseError('broken')
    this.step++
  }

  /** The session, taking messages up to `most` bytes from the other end. */
  session(most: number): Session {
    if (!this.finished || !this.remoteStatic) throw new NoiseError('out-of-turn')
    const [first, second] = this.state.split()
    const [sending, receiving] = this.initiator ? [first, second] : [second, first]
    this.ephemeral = null
    return new Session(sending, receiving, this.remoteStatic, most)
  }
}

export class Session {
  private readonly sending: CipherState
  private readonly receiving: CipherState
  private readonly most: number
  readonly peer: Uint8Array
  private inbox: Uint8Array[] = []
  private held = 0

  constructor(sending: CipherState, receiving: CipherState, peer: Uint8Array, most: number) {
    this.sending = sending
    this.receiving = receiving
    this.peer = peer
    this.most = most
  }

  /** `message` as the frames that carry it, in order. */
  seal(message: Uint8Array): Uint8Array[] {
    const frames: Uint8Array[] = []
    let at = 0
    do {
      const chunk = message.subarray(at, at + MOST_CHUNK)
      at += chunk.length
      const flag = at >= message.length ? LAST : MORE
      frames.push(this.sending.encrypt(new Uint8Array(0), concat(Uint8Array.of(flag), chunk)))
    } while (at < message.length)
    return frames
  }

  /** One frame in: the whole message once its last frame is, or null. */
  open(frame: Uint8Array): Uint8Array | null {
    if (frame.length > MOST_FRAME) throw new NoiseError('too-big')
    if (frame.length < TAG + 1) throw new NoiseError('broken')
    const plain = this.receiving.decrypt(new Uint8Array(0), frame)
    const flag = plain[0]
    const chunk = plain.subarray(1)
    // Checked before it is kept: the ceiling bounds what the page holds.
    if (this.held + chunk.length > this.most) throw new NoiseError('too-big')
    if (flag !== MORE && flag !== LAST) throw new NoiseError('broken')
    this.inbox.push(chunk)
    this.held += chunk.length
    if (flag === MORE) return null
    const whole = concat(...this.inbox)
    this.inbox = []
    this.held = 0
    return whole
  }
}

export function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
}

/** Bytes from hex, or null when it is not exactly that. */
export function unhex(text: string, length: number): Uint8Array | null {
  if (text.length !== length * 2 || !/^[0-9a-f]*$/i.test(text)) return null
  const out = new Uint8Array(length)
  for (let at = 0; at < length; at++) out[at] = parseInt(text.slice(at * 2, at * 2 + 2), 16)
  return out
}
