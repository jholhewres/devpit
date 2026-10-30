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
    /* The 95th batch in a hundred, not the worst one: a suite running beside
       this test can stall any single batch for a collection or a switch. */
    took.sort((a, b) => a - b)
    expect(took[Math.floor(took.length * 0.95)]).toBeLessThan(5)
  })
})
