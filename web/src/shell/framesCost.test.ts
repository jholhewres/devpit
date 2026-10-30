import { describe, expect, it } from 'vitest'

import type { Frame, Message } from '../gen/bindings'
import { applied, appliedAll } from './frames'

/* A long thread, and a long turn streaming into its end: text in small
   pieces between tool calls, the way a turn with tools arrives. */
function thread(size: number): Message[] {
  return Array.from({ length: size }, (_, at) => ({
    id: `m${at}`,
    turnId: null,
    role: at % 2 ? 'assistant' : 'user',
    parts: [{ kind: 'text', text: 'said '.repeat(40), parent: null }],
    createdAt: 0,
    streaming: false,
  })) as Message[]
}

function turn(pieces: number): Frame[] {
  const frames: Frame[] = [
    { type: 'opened', message: { id: 'live', turnId: 't', role: 'assistant', parts: [], createdAt: 0, streaming: true } } as Frame,
  ]
  for (let at = 0; at < pieces; at += 1) {
    if (at % 50 === 49) {
      frames.push({ type: 'part', message_id: 'live', part: { kind: 'tool_call', id: `c${at}`, name: 'Bash', input: '{}', state: 'running', parent: null } } as Frame)
      frames.push({ type: 'call_state', call_id: `c${at}`, state: 'ok' } as Frame)
    } else {
      frames.push({ type: 'part', message_id: 'live', part: { kind: 'text', text: 'word ', parent: null } } as Frame)
    }
  }
  return frames
}

describe('a long turn streaming into a long thread', () => {
  it('ends the same whichever way its frames are applied', () => {
    const start = thread(50)
    const frames = turn(500)
    const one = frames.reduce(applied, start)
    const batched = appliedAll(start, frames)
    expect(JSON.stringify(batched)).toBe(JSON.stringify(one))
  })

  it('leaves an answered message as it is, the same object, when a call moves', () => {
    const answered = {
      id: 'old', turnId: null, role: 'assistant', createdAt: 0, streaming: false,
      parts: [{ kind: 'tool_call', id: 'c1', name: 'Bash', input: '{}', state: 'ok', parent: null }],
    } as Message
    const live = appliedAll([answered], turn(60))
    const moved = appliedAll(live, [{ type: 'call_state', call_id: 'c1', state: 'failed' } as Frame])
    expect(moved[0]).toBe(answered)
  })

  it('looks at each message of the thread about once a batch, not once a frame', () => {
    /* Every read of an answered message's fields is counted. */
    let reads = 0
    const watched = thread(2000).map(
      (message) => new Proxy(message, { get: (target, key, receiver) => ((reads += 1), Reflect.get(target, key, receiver)) }),
    )
    const start = appliedAll(watched, turn(1).slice(0, 1))
    reads = 0
    appliedAll(start, turn(60).slice(1))
    // A walk is 2000 reads, and each tool call's update is one; a walk per frame would be 60.
    const calls = turn(60).filter((frame) => frame.type === 'call_state').length
    expect(reads).toBeLessThan((calls + 1) * 2000)
  })

  it('copies the thread once a batch, not once a frame', () => {
    const start = appliedAll(thread(2000), turn(1).slice(0, 1))
    /* Every copy of a long list iterates it; count those of the thread's length or more. */
    const iterate = Array.prototype[Symbol.iterator]
    let copies = 0
    Array.prototype[Symbol.iterator] = function (this: unknown[]) {
      if (this.length >= 2000) copies += 1
      return iterate.call(this)
    }
    try {
      appliedAll(start, turn(60).slice(1))
    } finally {
      Array.prototype[Symbol.iterator] = iterate
    }
    expect(copies).toBe(1)
  })

  it('joins the text that runs on once a batch, not once a piece', () => {
    const live = appliedAll([], [turn(1)[0]!, { type: 'part', message_id: 'live', part: { kind: 'text', text: 'first ', parent: null } } as Frame])
    /* Every join rebuilds the live message's parts without their last; count those. */
    const slice = Array.prototype.slice
    let rebuilt = 0
    Array.prototype.slice = function (this: unknown[], ...range: [number?, number?]) {
      if (range[0] === 0 && range[1] === -1) rebuilt += 1
      return slice.apply(this, range)
    }
    const pieces = Array.from({ length: 40 }, () => ({ type: 'part', message_id: 'live', part: { kind: 'text', text: 'word ', parent: null } }) as Frame)
    let after: readonly Message[]
    try {
      after = appliedAll(live, pieces)
    } finally {
      Array.prototype.slice = slice
    }
    expect(after.at(-1)?.parts).toEqual([{ kind: 'text', text: `first ${'word '.repeat(40)}`, parent: null }])
    expect(rebuilt).toBe(1)
  })

  it('costs each batch of frames well under five milliseconds on 2000 messages', () => {
    const frames = turn(4000)
    let messages: readonly Message[] = appliedAll(thread(2000), frames.slice(0, 1))
    // Batches as a paint delivers them: about a dozen frames at a time.
    const took: number[] = []
    for (let at = 1; at < frames.length; at += 12) {
      const t0 = performance.now()
      messages = appliedAll(messages, frames.slice(at, at + 12))
      took.push(performance.now() - t0)
    }
    expect(messages.at(-1)?.parts.length).toBeGreaterThan(1)
    /* The middle batch, not the slowest few: on a loaded machine a suite
       running beside this test stalls whole runs of batches for collections
       and switches. What a batch copies and joins is counted exactly above;
       this only says the usual batch stays cheap. */
    took.sort((a, b) => a - b)
    expect(took[Math.floor(took.length / 2)]).toBeLessThan(5)
  })
})
