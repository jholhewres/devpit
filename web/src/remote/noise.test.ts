import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import { Handshake, hex, keyPair, MOST_CHUNK, MOST_FRAME, NoiseError, type Session, unhex } from './noise'

/* The transcript snow made with every key fixed. The machine's Rust is held
   to it in crates/remote/tests/the_noise_vector_holds.rs; this holds the
   viewer to it, byte for byte. */
interface Vector {
  pattern: string
  host: string
  initiator: { static: string; ephemeral: string; public: string }
  responder: { static: string; ephemeral: string; public: string }
  handshake: [string, string, string]
  frames: { from: 'initiator' | 'responder'; message: string; frame: string }[]
}

/* From the web root, as the other fixtures: under jsdom `import.meta.url` is http. */
const vector = JSON.parse(readFileSync(resolve(process.cwd(), '../crates/remote/tests/noise-xx.json'), 'utf8')) as Vector

const bytes = (text: string): Uint8Array => unhex(text, text.length / 2)!
const words = new TextEncoder()
const read = new TextDecoder()

function pair(host = 'host_01JSTEST', most = 1 << 20): { device: Session; machine: Session } {
  const device = new Handshake(true, keyPair(), host)
  const machine = new Handshake(false, keyPair(), host)
  machine.read(device.write())
  device.read(machine.write())
  machine.read(device.write())
  return { device: device.session(most), machine: machine.session(most) }
}

describe('the viewer’s Noise', () => {
  it('speaks the conversation snow made, byte for byte', () => {
    expect(vector.pattern).toBe('Noise_XX_25519_ChaChaPoly_BLAKE2s')
    const keys = (side: Vector['initiator']) => ({ local: keyPair(bytes(side.static)), ephemeral: keyPair(bytes(side.ephemeral)) })
    const [ours, theirs] = [keys(vector.initiator), keys(vector.responder)]
    const device = new Handshake(true, ours.local, vector.host, ours.ephemeral)
    const machine = new Handshake(false, theirs.local, vector.host, theirs.ephemeral)

    const first = device.write()
    expect(hex(first)).toBe(vector.handshake[0])
    machine.read(first)
    const second = machine.write()
    expect(hex(second)).toBe(vector.handshake[1])
    device.read(second)
    expect(hex(device.peer!)).toBe(vector.responder.public)
    const third = device.write()
    expect(hex(third)).toBe(vector.handshake[2])
    machine.read(third)
    expect(hex(machine.peer!)).toBe(vector.initiator.public)

    const ends = { initiator: device.session(1 << 20), responder: machine.session(1 << 20) }
    for (const one of vector.frames) {
      const [by, to] = one.from === 'initiator' ? [ends.initiator, ends.responder] : [ends.responder, ends.initiator]
      const frames = by.seal(words.encode(one.message))
      expect(frames.map(hex)).toEqual([one.frame])
      expect(read.decode(to.open(bytes(one.frame))!)).toBe(one.message)
    }
  })

  it('carries a message longer than a frame in frames, whole only at the last', () => {
    const { device, machine } = pair()
    const board = new Uint8Array(MOST_CHUNK * 2 + 5).fill(120)
    const frames = machine.seal(board)
    expect(frames).toHaveLength(3)
    expect(frames.every((frame) => frame.length <= MOST_FRAME)).toBe(true)
    expect(device.open(frames[0]!)).toBeNull()
    expect(device.open(frames[1]!)).toBeNull()
    expect(device.open(frames[2]!)).toEqual(board)
  })

  it('refuses a frame changed, dropped or repeated on the way', () => {
    const fault = (run: () => unknown): string | null => {
      try {
        run()
        return null
      } catch (err) {
        return err instanceof NoiseError ? err.fault : 'other'
      }
    }
    let { device, machine } = pair()
    const changed = device.seal(words.encode('type ls'))[0]!
    changed[2]! ^= 1
    expect(fault(() => machine.open(changed))).toBe('broken')
    ;({ device, machine } = pair())
    device.seal(words.encode('one'))
    expect(fault(() => machine.open(device.seal(words.encode('two'))[0]!))).toBe('broken')
    ;({ device, machine } = pair())
    const once = device.seal(words.encode('once'))[0]!
    expect(read.decode(machine.open(once)!)).toBe('once')
    expect(fault(() => machine.open(once))).toBe('broken')
  })

  it('refuses a message past its ceiling before keeping it', () => {
    const { device, machine } = pair('host_01JSTEST', MOST_CHUNK + 10)
    const frames = machine.seal(new Uint8Array(MOST_CHUNK * 2))
    expect(device.open(frames[0]!)).toBeNull()
    expect(() => device.open(frames[1]!)).toThrow(NoiseError)
    expect(() => device.open(new Uint8Array(MOST_FRAME + 1))).toThrow(/too-big/)
  })

  it('does not finish a handshake meant for another machine', () => {
    const device = new Handshake(true, keyPair(), 'host_01ONE')
    const machine = new Handshake(false, keyPair(), 'host_01OTHER')
    machine.read(device.write())
    expect(() => device.read(machine.write())).toThrow(/broken/)
  })
})
