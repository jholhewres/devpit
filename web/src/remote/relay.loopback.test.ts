// @vitest-environment node
import { type ChildProcessWithoutNullStreams, spawn } from 'node:child_process'
import { mkdtempSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterAll, beforeAll, describe, expect, it } from 'vitest'

import type { RemoteOut } from '../gen/bindings'
import { Handshake, hex, keyPair, NoiseError } from './noise'
import { CLOSED, connectRelay, type RelayMachine } from './relay'

/*
 * The Remote, end to end, without Tailscale: devpit-app's relay, the
 * machine's relay client (crates/remote, as the desktop runs it), and this
 * viewer's own relay transport — all on loopback, every frame the relay
 * carried captured to a file.
 *
 * It needs a devpit-app checkout and builds both sides, so it runs only when
 * asked: DEVPIT_APP_DIR=../devpit-app pnpm --filter ./web exec vitest run
 * src/remote/relay.loopback.test.ts
 */

const APP = process.env.DEVPIT_APP_DIR
const ROOT = resolve(process.cwd(), '..')

interface RelayInfo {
  relay: string
  machine: string
  device: string
  stranger: string
}

interface HostInfo {
  host: string
  key: string
  secret: string
}

function firstLine(child: ChildProcessWithoutNullStreams, what: string): Promise<string> {
  return new Promise((done, fail) => {
    let out = ''
    let err = ''
    child.stdout.on('data', (chunk: Buffer) => {
      out += chunk.toString()
      const line = out.split('\n')[0]
      if (out.includes('\n') && line) done(line)
    })
    child.stderr.on('data', (chunk: Buffer) => (err += chunk.toString()))
    child.on('exit', (code) => fail(new Error(`${what} exited ${code}: ${err.slice(-2000)}`)))
  })
}

function until<T>(find: () => T | undefined, within: number, what: string): Promise<T> {
  return new Promise((done, fail) => {
    const began = Date.now()
    const look = (): void => {
      const found = find()
      if (found !== undefined) return done(found)
      if (Date.now() - began > within) return fail(new Error(`no ${what} within ${within} ms`))
      setTimeout(look, 25)
    }
    look()
  })
}

const opener = (token: string) => (url: string) => new WebSocket(url, { headers: { authorization: `Bearer ${token}` } } as unknown as string[])

describe.skipIf(!APP)('the Remote through a local devpit-app relay', () => {
  let relayProcess: ChildProcessWithoutNullStreams
  let hostProcess: ChildProcessWithoutNullStreams
  let relay: RelayInfo
  let host: HostInfo
  let capture: string

  beforeAll(async () => {
    capture = join(mkdtempSync(join(tmpdir(), 'devpit-relay-')), 'frames.txt')
    relayProcess = spawn('cargo', ['run', '-q', '-p', 'devpit-app-relay', '--example', 'loopback', '--', capture], { cwd: APP })
    relay = JSON.parse(await firstLine(relayProcess, 'the relay')) as RelayInfo
    hostProcess = spawn('cargo', ['run', '-q', '-p', 'devpit-remote', '--example', 'loopback_host', '--', relay.relay, relay.machine], { cwd: ROOT })
    host = JSON.parse(await firstLine(hostProcess, 'the machine')) as HostInfo
  }, 900_000)

  afterAll(() => {
    hostProcess?.kill()
    relayProcess?.kill()
  })

  it('opens, pairs, carries what the relay cannot read, and drops a device the moment it is unpaired', async () => {
    const machine: RelayMachine = { host: host.host, key: host.key, name: 'loopback' }
    const heard: RemoteOut[] = []
    const states: { up: boolean; code?: number }[] = []
    const link = connectRelay(machine, keyPair(), (said) => heard.push(said), (up, _why, code) => states.push({ up, code }), {
      origin: relay.relay,
      pairing: { secret: host.secret, name: 'Loopback phone' },
      open: opener(relay.device),
    })

    // Paired through the relay, then answered like any viewer.
    await until(() => heard.find((said) => said.t === 'welcome'), 20_000, 'welcome')
    link.send({ t: 'projects' })
    const projects = await until(() => heard.find((said) => said.t === 'projects'), 10_000, 'projects')
    expect(projects.t === 'projects' && projects.projects[0]?.name).toBe('loopback secret project')
    // A message longer than one Noise frame, from Rust to the page.
    link.send({ t: 'drafts' })
    const drafts = await until(() => heard.find((said) => said.t === 'drafts'), 10_000, 'drafts')
    expect(drafts.t === 'drafts' && drafts.drafts[0]?.text.length).toBe(150_000)

    // What the relay carried: ciphertext, which keys it could make open none of.
    const frames = readFileSync(capture, 'utf8')
      .trim()
      .split('\n')
      .map((line) => line.split(' ') as [string, string])
    expect(frames.length).toBeGreaterThan(6)
    const plain = (text: string): string => hex(new TextEncoder().encode(text))
    for (const [, bytes] of frames) {
      for (const words of ['loopback secret project', host.secret, 'Loopback phone', 'welcome', 'xxxxxxxx']) {
        expect(bytes.includes(plain(words))).toBe(false)
      }
    }
    const outsider = new Handshake(true, keyPair(), host.host)
    const other = new Handshake(false, keyPair(), host.host)
    other.read(outsider.write())
    outsider.read(other.write())
    other.read(outsider.write())
    const [one, two] = [outsider.session(1 << 24), other.session(1 << 24)]
    for (const [, bytes] of frames.slice(3)) {
      const frame = Uint8Array.from(bytes.match(/../g)!.map((byte) => parseInt(byte, 16)))
      expect(() => one.open(frame)).toThrow(NoiseError)
      expect(() => two.open(frame)).toThrow(NoiseError)
    }

    // Unpaired on the machine: its side closes, the relay closes this one at
    // once, and the next handshake is refused.
    const before = states.length
    hostProcess.stdin.write('forget\n')
    const dropped = await until(() => states.slice(before).find((state) => !state.up), 2_000, 'the drop')
    expect(dropped.code).toBe(4408)
    const refused = await until(() => heard.find((said) => said.t === 'refused'), 15_000, 'the refusal')
    expect(refused.t === 'refused' && refused.why).toMatch(/not paired/)
    link.close()
  }, 60_000)

  it('joins a device to no machine of another account', async () => {
    const machine: RelayMachine = { host: host.host, key: host.key, name: 'loopback' }
    const states: { up: boolean; code?: number }[] = []
    const link = connectRelay(machine, keyPair(), () => undefined, (up, _why, code) => states.push({ up, code }), {
      origin: relay.relay,
      open: opener(relay.stranger),
    })
    const refused = await until(() => states.find((state) => state.code !== undefined), 10_000, 'a close')
    expect(refused.code).toBe(CLOSED.offline)
    link.close()
  }, 30_000)
})
