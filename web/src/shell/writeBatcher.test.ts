import { describe, expect, it } from 'vitest'

import type { Screen } from './floodCut'
import { ACK_EVERY, writeBatcher } from './writeBatcher'

/* A terminal that parses on demand, and a frame clock moved by hand. */
function setup(screen: Screen = null) {
  const writes: number[][] = []
  const parsing: (() => void)[] = []
  const term = {
    write: (data: Uint8Array, done?: () => void) => {
      writes.push([...data])
      if (done) parsing.push(done)
    },
  }
  let frame: (() => void) | null = null
  const acks: number[] = []
  const batcher = writeBatcher(term, (bytes) => acks.push(bytes), () => screen, {
    frame: (run) => {
      frame = run
      return () => (frame = null)
    },
  })
  return {
    batcher,
    writes,
    acks,
    parse: () => parsing.splice(0).forEach((done) => done()),
    tick: () => {
      const run = frame
      frame = null
      run?.()
    },
    scheduled: () => frame !== null,
  }
}

const bytes = (n: number, fill = 1) => new Uint8Array(n).fill(fill)

describe('the frame batcher in front of a terminal', () => {
  it('writes a keystroke echo at once when nothing is waiting', () => {
    const t = setup()
    t.batcher.push(bytes(3))
    expect(t.writes).toEqual([[1, 1, 1]])
    expect(t.scheduled()).toBe(false)
  })

  it('holds a big chunk for the next frame', () => {
    const t = setup()
    t.batcher.push(bytes(513))
    expect(t.writes).toEqual([])
    t.tick()
    expect(t.writes.map((w) => w.length)).toEqual([513])
  })

  it('writes everything that came in one frame as one write, in order', () => {
    const t = setup()
    t.batcher.push(bytes(600, 1))
    t.batcher.push(bytes(2, 2))
    t.batcher.push(bytes(700, 3))
    expect(t.writes).toEqual([])
    t.tick()
    expect(t.writes).toHaveLength(1)
    expect(t.writes[0]).toEqual([...bytes(600, 1), ...bytes(2, 2), ...bytes(700, 3)])
  })

  it('queues a small chunk behind a waiting one rather than jumping it', () => {
    const t = setup()
    t.batcher.push(bytes(600, 1))
    t.batcher.push(bytes(1, 9))
    t.tick()
    expect(t.writes).toEqual([[...bytes(600, 1), 9]])
  })

  it('acknowledges what the terminal parsed, grouped every 64 KB', () => {
    const t = setup()
    expect(ACK_EVERY).toBe(64 * 1024)
    t.batcher.push(bytes(40 * 1024))
    t.tick()
    t.parse()
    expect(t.acks).toEqual([])
    t.batcher.push(bytes(30 * 1024))
    t.tick()
    expect(t.acks).toEqual([])
    t.parse()
    expect(t.acks).toEqual([70 * 1024])
  })

  it('flushes what waits on demand, and the frame then has nothing to do', () => {
    const t = setup()
    t.batcher.push(bytes(600))
    t.batcher.flush()
    expect(t.writes.map((w) => w.length)).toEqual([600])
    expect(t.scheduled()).toBe(false)
  })

  /* The pty is paced by acks: bytes a cut took out were never parsed, but
     acknowledging less would hold the pane for good. */
  it('acknowledges a flood whole, the bytes the cut took out included', () => {
    const t = setup({ rows: 24, scrollback: 1000 })
    const flood = new TextEncoder().encode('y\r\n'.repeat(30_000))
    t.batcher.push(flood)
    t.tick()
    t.parse()
    expect(t.writes.flat().length).toBeLessThan(flood.length / 2)
    expect(t.acks).toEqual([flood.length])
  })

  it('writes nothing and acknowledges nothing once disposed', () => {
    const t = setup()
    t.batcher.push(bytes(70 * 1024))
    t.batcher.dispose()
    expect(t.scheduled()).toBe(false)
    t.tick()
    t.parse()
    expect(t.writes).toEqual([])
    expect(t.acks).toEqual([])
  })
})
