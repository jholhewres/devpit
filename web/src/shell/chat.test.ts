import { describe, expect, it } from 'vitest'

import type { Attachment, Conversation, Frame, Message, Profile } from '../gen/bindings'
import {
  applied,
  ASKS,
  choices,
  fixedTo,
  effortName,
  MODES,
  modelName,
  modeName,
  money,
  ready,
  withFiles,
} from './chat'

const message = (id: string, streaming = true): Message =>
  ({ id, turnId: null, role: 'assistant', parts: [], createdAt: 0, streaming })

const opened = (id: string): Frame => ({ type: 'opened', message: message(id) })
const text = (id: string, t: string): Frame =>
  ({ type: 'part', message_id: id, part: { kind: 'text', text: t } } as Frame)

describe('what arrives on the stream', () => {
  it('adds the message that opened', () => {
    expect(applied([], opened('m1')).map((m) => m.id)).toEqual(['m1'])
  })

  it('grows the open message rather than adding another', () => {
    const after = applied(applied([], opened('m1')), text('m1', 'hi'))
    expect(after).toHaveLength(1)
    expect(after[0]!.parts).toEqual([{ kind: 'text', text: 'hi' }])
  })

  it('joins consecutive text into one paragraph', () => {
    /* One part per chunk would draw a new block for every few characters. */
    let msgs = applied([], opened('m1'))
    msgs = applied(msgs, text('m1', 'he'))
    msgs = applied(msgs, text('m1', 'llo'))
    expect(msgs[0]!.parts).toEqual([{ kind: 'text', text: 'hello' }])
  })

  it('keeps a tool call separate from the text around it', () => {
    let msgs = applied([], opened('m1'))
    msgs = applied(msgs, text('m1', 'running'))
    msgs = applied(msgs, {
      type: 'part',
      message_id: 'm1',
      part: { kind: 'tool_call', id: 'c1', name: 'Bash', input: '{}', state: 'running' },
    } as Frame)
    expect(msgs[0]!.parts).toHaveLength(2)
  })

  it('moves a call on without touching the rest', () => {
    let msgs = applied([], opened('m1'))
    msgs = applied(msgs, {
      type: 'part',
      message_id: 'm1',
      part: { kind: 'tool_call', id: 'c1', name: 'Bash', input: '{}', state: 'running' },
    } as Frame)
    msgs = applied(msgs, { type: 'call_state', call_id: 'c1', state: 'ok' } as Frame)
    expect(msgs[0]!.parts[0]).toMatchObject({ kind: 'tool_call', state: 'ok' })
  })

  it('stops the spinner when the turn ends', () => {
    /* The end is an event; the absence of frames is not an ending. */
    const msgs = applied(applied([], opened('m1')), {
      type: 'ended',
      end: { turnId: 't', costUsd: null, durationMs: null, stopReason: null, isError: false },
    } as Frame)
    expect(msgs[0]!.streaming).toBe(false)
  })

  it('ignores a frame for a message it does not have', () => {
    expect(applied([], text('nope', 'x'))).toEqual([])
  })
})

describe('which profile the composer may use', () => {
  const profile = (id: string, path: string | null): Profile => ({
    id,
    label: id,
    command: id,
    driver: 'claude',
    path,
    models: ['default'],
  })

  it('offers every installed profile before the first turn', () => {
    const all = [profile('claude', '/usr/bin/claude'), profile('claudin', '/usr/bin/claudin')]
    expect(choices(all, null).map((one) => one.id)).toEqual(['claude', 'claudin'])
  })

  it('leaves out a profile that is not on the PATH', () => {
    const all = [profile('claude', '/usr/bin/claude'), profile('codex', null)]
    expect(choices(all, null).map((one) => one.id)).toEqual(['claude'])
  })

  it('offers only the one the conversation belongs to', () => {
    const all = [profile('claude', '/usr/bin/claude'), profile('claudin', '/usr/bin/claudin')]
    expect(choices(all, 'claudin').map((one) => one.id)).toEqual(['claudin'])
  })

  it('reads a conversation with no turns as belonging to nobody', () => {
    expect(fixedTo({ profile: '' } as Conversation)).toBeNull()
  })

  it('reads a conversation that has spoken as fixed', () => {
    expect(fixedTo({ profile: 'claudin' } as Conversation)).toBe('claudin')
  })
})

describe('whether the send button does anything', () => {
  it('does nothing while a turn is running', () => {
    expect(ready('hello', true, 'claude')).toBe(false)
  })

  it('does nothing with nothing to say', () => {
    expect(ready('   ', false, 'claude')).toBe(false)
  })

  it('does nothing with no profile picked', () => {
    expect(ready('hello', false, null)).toBe(false)
  })

  it('sends when there is a profile and something to say', () => {
    expect(ready('hello', false, 'claude')).toBe(true)
  })
})

describe('what a conversation cost', () => {
  it('says nothing before anything was spent', () => {
    expect(money(0)).toBeNull()
  })

  it('keeps four places while the number is smaller than a cent', () => {
    expect(money(0.0042)).toBe('$0.0042')
  })

  it('rounds to cents once there are cents to round', () => {
    expect(money(1.239)).toBe('$1.24')
  })
})

describe('what the agent is actually sent', () => {
  const file = (path: string): Attachment => ({ name: path, path, kind: 'rs' })

  it('sends the prompt untouched when nothing is attached', () => {
    expect(withFiles('why is this slow?', [])).toBe('why is this slow?')
  })

  it('names each attached file as a path the agent can open', () => {
    expect(withFiles('why?', [file('src/a.rs'), file('src/b.rs')])).toBe(
      '@src/a.rs @src/b.rs\nwhy?',
    )
  })
})

describe('what the agent may do without asking', () => {
  it('offers the mode that stops to ask, now that there is somewhere to answer', () => {
    expect(MODES.map((mode) => mode.id)).toContain('manual')
  })

  it('knows which mode stops to ask', () => {
    expect(ASKS('manual')).toBe(true)
    expect(ASKS('acceptEdits')).toBe(false)
    expect(ASKS('bypassPermissions')).toBe(false)
  })
})

describe('what a model is called on screen', () => {
  /* A fine argument to the CLI, and a label that names no model. */
  it('says whose default it is rather than the word default', () => {
    expect(modelName('default')).toBe("The account's default")
  })

  it('leaves a real model name alone', () => {
    expect(modelName('opus')).toBe('opus')
  })
})

describe('what a permission mode says it does', () => {
  /* "Ask" names nothing on its own; the row exists so you know what you are
     agreeing to. */
  it('every mode explains itself', () => {
    for (const mode of MODES) {
      expect(mode.what.length).toBeGreaterThan(10)
    }
  })

  it('names a mode by its label rather than the CLI token', () => {
    expect(modeName('bypassPermissions')).toBe('Full access')
    expect(modeName('manual')).toBe('Supervised')
  })
})

describe('how hard the agent is asked to think', () => {
  it('says Extra high rather than xhigh, which is an argument', () => {
    expect(effortName('xhigh')).toBe('Extra high')
  })

  it('leaves a level it does not know alone', () => {
    expect(effortName('ultracode')).toBe('ultracode')
  })
})
