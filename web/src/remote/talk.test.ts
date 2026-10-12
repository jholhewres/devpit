import { describe, expect, it } from 'vitest'

import type { Conversation, Message, RemoteDraft, RemoteIn, RemoteOut } from '../gen/bindings'
import { after, draftsOf, photoRefused, pieces, PIECE, said, voiceRefused, type Talking } from './talk'

const orchestrator: Talking = { kind: 'chat', project: 'orch', conversation: 'c1', title: 'work', orchestrator: 'claude-work' }
const session: Talking = { kind: 'session', profile: 'claude-work', name: 'api-worker', title: 'api-worker', project: 'api', pane: null }

const message = (id: string, role: Message['role'], text: string, streaming = false): Message => ({
  id,
  turnId: 't1',
  role,
  parts: text ? [{ kind: 'text', text, parent: null }] : [],
  createdAt: 0,
  streaming,
})

const history = (messages: Message[], id = 'c1'): RemoteOut => ({ t: 'chat', project: 'orch', conversation: { id, messages } as unknown as Conversation })

describe('what the chat view shows', () => {
  it('is the conversation read, then each frame of a turn as it streams', () => {
    let shown = after([], orchestrator, history([message('m1', 'user', 'how is the api?')]))
    expect(shown).toHaveLength(1)
    const frame = (frame: Extract<RemoteOut, { t: 'chatFrame' }>['frame']): RemoteOut => ({ t: 'chatFrame', project: 'orch', conversation: 'c1', frame })
    shown = after(shown, orchestrator, frame({ type: 'opened', message: message('m2', 'assistant', '', true) }))
    shown = after(shown, orchestrator, frame({ type: 'part', message_id: 'm2', part: { kind: 'text', text: 'It is ', parent: null } }))
    shown = after(shown, orchestrator, frame({ type: 'part', message_id: 'm2', part: { kind: 'text', text: 'green.', parent: null } }))
    expect(shown[1]!.parts).toEqual([{ kind: 'text', text: 'It is green.', parent: null }])
    expect(shown[1]!.streaming).toBe(true)
    // Read again mid-turn, the transcript has none of it yet: the frames stay.
    expect(after(shown, orchestrator, history([message('m1', 'user', 'how is the api?')]))).toBe(shown)
    shown = after(shown, orchestrator, frame({ type: 'ended', end: { turnId: 't1', costUsd: null, durationMs: null, stopReason: null, isError: false, context: null } }))
    expect(shown[1]!.streaming).toBe(false)
    // Another conversation's words are not this one's.
    expect(after(shown, orchestrator, history([], 'c2'))).toBe(shown)
  })

  it('is a session read from its transcript', () => {
    const read: RemoteOut = { t: 'session', profile: 'claude-work', name: 'api-worker', messages: [message('s1', 'assistant', 'done')] }
    expect(after([], session, read)).toHaveLength(1)
    expect(after([], orchestrator, read)).toHaveLength(0)
  })

  it('sends the words to the conversation or into the session, and keeps the drafts that belong to it', () => {
    expect(said(orchestrator, 'go on')).toEqual({ t: 'chatSay', project: 'orch', conversation: 'c1', text: 'go on' })
    expect(said(session, 'go on')).toEqual({ t: 'sessionSay', profile: 'claude-work', name: 'api-worker', text: 'go on' })
    const drafts: RemoteDraft[] = [
      { profile: 'claude-work', name: 'api-worker', text: 'yes' },
      { profile: 'claude-work', name: 'site', text: 'no' },
      { profile: 'claude-home', name: 'api-worker', text: 'other account' },
    ]
    expect(draftsOf(orchestrator, drafts)).toHaveLength(2)
    expect(draftsOf(session, drafts)).toEqual([drafts[0]])
    expect(draftsOf({ ...orchestrator, orchestrator: null }, drafts)).toEqual([])
  })
})

describe('a file from the phone', () => {
  it('goes in pieces each far under what the machine takes in one message', () => {
    const bytes = new Uint8Array(PIECE * 2 + 10).map((_, at) => at % 251)
    const finish: RemoteIn = { t: 'voice', upload: 'u1', mime: 'audio/mp4' }
    const sent = pieces(bytes, 'u1', finish)
    expect(sent).toHaveLength(4)
    expect(sent.at(-1)).toBe(finish)
    for (const one of sent) expect(JSON.stringify(one).length).toBeLessThan(256 * 1024)
    const back = sent.flatMap((one) => (one.t === 'uploadPart' ? [...atob(one.b64)].map((c) => c.charCodeAt(0)) : []))
    expect(back).toEqual([...bytes])
  })

  it('is refused here when it is too big or not a photo or a recording', () => {
    expect(photoRefused(new Blob([new Uint8Array(1000)], { type: 'image/jpeg' }))).toBeNull()
    expect(photoRefused(new Blob([new Uint8Array(8 * 1024 * 1024 + 1)], { type: 'image/png' }))).toMatch(/over 8 MB/)
    expect(photoRefused(new Blob(['<svg/>'], { type: 'image/svg+xml' }))).toMatch(/PNG, JPEG/)
    expect(voiceRefused(new Blob([new Uint8Array(1000)], { type: 'audio/webm;codecs=opus' }))).toBeNull()
    expect(voiceRefused(new Blob([new Uint8Array(3 * 1024 * 1024 + 1)], { type: 'audio/mp4' }))).toMatch(/at most 3 MB/)
    expect(voiceRefused(new Blob(['x'], { type: 'video/mp4' }))).toMatch(/not a recording/)
  })
})
