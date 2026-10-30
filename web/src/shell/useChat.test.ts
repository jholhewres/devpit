import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Frame, Message, TurnEnd } from '../gen/bindings'
import { useChat } from './useChat'

afterEach(() => {
  cleanup()
  joins.length = 0
  woke.handler = null
})

/* What each rejoin was handed, and whether it was left. */
const joins: {
  conversationId: string
  onFrame: (frame: Frame) => void
  leave: ReturnType<typeof vi.fn>
  ends: (was: boolean) => void
}[] = []
const woke: { handler: ((conversationId: string) => void) | null } = { handler: null }
let ending: (end: TurnEnd) => void = () => {}

vi.mock('./chat', async (actual) => ({
  ...(await actual<typeof import('./chat')>()),
  rejoin: (conversationId: string, onFrame: (frame: Frame) => void) => {
    const leave = vi.fn()
    let ends: (was: boolean) => void = () => {}
    const running = new Promise<boolean>((resolve) => (ends = resolve))
    joins.push({ conversationId, onFrame, leave, ends })
    return { running, leave }
  },
  send: () => ({ end: new Promise<TurnEnd>((resolve) => (ending = resolve)) }),
}))
const message = (id: string, text = id): Message =>
  ({ id, turnId: null, role: 'assistant', parts: [{ kind: 'text', text, parent: null }], createdAt: 0, streaming: false }) as Message

/* What the transcript holds, and what it was asked. */
const saved: { messages: Message[]; after: Message[] } = { messages: [], after: [] }
const calls = {
  chatHistory: vi.fn(),
  chatHead: vi.fn(),
  chatAfter: vi.fn(),
  chatRewind: vi.fn(),
}
afterEach(() => {
  saved.messages = []
  saved.after = []
  Object.values(calls).forEach((one) => one.mockClear())
})

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    chatHistory: (projectId: string, id: string) => (
      calls.chatHistory(projectId, id),
      {
        id,
        projectId,
        messages: saved.messages,
        profile: 'claude',
        rewindable: [],
        sessionId: null,
        costUsd: 0,
        cardId: null,
      }
    ),
    chatHead: (projectId: string, id: string) => (
      calls.chatHead(projectId, id),
      { sessionId: 'ses_1', rewindable: ['turn_1'], costUsd: 0.5, context: null }
    ),
    chatAfter: (projectId: string, id: string, after: string | null) => (
      calls.chatAfter(projectId, id, after),
      { messages: saved.after }
    ),
    chatRewind: (projectId: string, id: string, turnId: string) => (calls.chatRewind(projectId, id, turnId), 'conv_fork'),
    agentProfiles: () => [{ id: 'claude', path: '/bin/claude' }],
    permissionAskFromNow: () => null,
  },
}))
vi.mock('./window', () => ({
  inTauri: () => true,
  onChatWoke: (then: (conversationId: string) => void) => ((woke.handler = then), () => {}),
  onPermissionAsked: () => () => {},
  onPermissionSettled: () => () => {},
}))
const shell = { project: { id: 'p', rootPath: '/w', orchestrator: null }, show: vi.fn() }
vi.mock('./shellStore', () => ({ useShellPick: (pick: (from: typeof shell) => unknown) => pick(shell) }))

const said = (id: string): Frame => ({
  type: 'opened',
  message: { id, turnId: null, role: 'assistant', parts: [], createdAt: 0, streaming: true } as Message,
})

describe("a chat's joined turns", () => {
  it('write nothing into the next conversation shown in the same chat', async () => {
    const { result, rerender } = renderHook(({ id }) => useChat(id), { initialProps: { id: 'conv_a' } })
    await waitFor(() => expect(joins.map((one) => one.conversationId)).toEqual(['conv_a']))
    rerender({ id: 'conv_b' })
    await waitFor(() => expect(joins.map((one) => one.conversationId)).toEqual(['conv_a', 'conv_b']))

    act(() => {
      joins[0]!.onFrame(said('msg_from_a'))
      joins[1]!.onFrame(said('msg_from_b'))
    })
    await waitFor(() => expect(result.current.messages.map((one) => one.id)).toContain('msg_from_b'))
    expect(result.current.messages.map((one) => one.id)).not.toContain('msg_from_a')
    expect(joins[0]!.leave).toHaveBeenCalled()
  })

  it('are left when the chat goes', async () => {
    const { unmount } = renderHook(() => useChat('conv_a'))
    await waitFor(() => expect(joins).toHaveLength(1))
    unmount()
    expect(joins[0]!.leave).toHaveBeenCalled()
  })

  it('take a woken turn after the chat’s own, which would otherwise be heard twice', async () => {
    const { result } = renderHook(() => useChat('conv_a'))
    await waitFor(() => expect(result.current.profileId).toBe('claude'))
    await waitFor(() => expect(joins).toHaveLength(1))
    act(() => result.current.say('hello'))

    act(() => woke.handler?.('conv_a'))
    expect(joins).toHaveLength(1)
    await act(async () =>
      ending({ turnId: 't', costUsd: null, durationMs: null, stopReason: null, isError: false, context: null }),
    )
    await waitFor(() => expect(joins).toHaveLength(2))
  })
})

describe("a chat's reads of the transcript", () => {
  const end: TurnEnd = { turnId: 't', costUsd: null, durationMs: null, stopReason: null, isError: false, context: null }

  it('at the end of a turn are of its head, not of the whole conversation', async () => {
    const { result } = renderHook(() => useChat('conv_a'))
    await waitFor(() => expect(result.current.profileId).toBe('claude'))
    expect(calls.chatHistory).toHaveBeenCalledTimes(1)
    act(() => result.current.say('hello'))
    await act(async () => ending(end))
    await waitFor(() => expect(calls.chatHead).toHaveBeenCalledWith('p', 'conv_a'))
    await waitFor(() => expect(result.current.rewindable).toEqual(['turn_1']))
    expect(calls.chatHistory).toHaveBeenCalledTimes(1)
  })

  it('after a rejoined turn append what came after the last message, leaving the rest as drawn', async () => {
    saved.messages = [message('msg_1'), message('msg_2')]
    const { result } = renderHook(() => useChat('conv_a'))
    await waitFor(() => expect(joins).toHaveLength(1))
    await waitFor(() => expect(result.current.messages.map((one) => one.id)).toEqual(['msg_1', 'msg_2']))
    const before = result.current.messages
    act(() => joins[0]!.onFrame(said('msg_3')))
    await waitFor(() => expect(result.current.messages).toHaveLength(3))

    saved.after = [message('msg_3', 'as saved'), message('msg_4')]
    await act(async () => joins[0]!.ends(true))
    await waitFor(() => expect(result.current.messages.map((one) => one.id)).toEqual(['msg_1', 'msg_2', 'msg_3', 'msg_4']))
    expect(calls.chatAfter).toHaveBeenCalledWith('p', 'conv_a', 'msg_2')
    expect(calls.chatHistory).toHaveBeenCalledTimes(1)
    // Not replaced: what was drawn before the join is the same objects.
    expect(result.current.messages[0]).toBe(before[0])
    expect(result.current.messages[1]).toBe(before[1])
    expect(result.current.messages[2]!.parts).toEqual([{ kind: 'text', text: 'as saved', parent: null }])
  })

  it('after a woken turn append from the last message drawn, not from the start', async () => {
    saved.messages = [message('msg_1'), message('msg_2')]
    const { result } = renderHook(() => useChat('conv_a'))
    await waitFor(() => expect(result.current.messages.map((one) => one.id)).toEqual(['msg_1', 'msg_2']))
    const before = result.current.messages
    act(() => woke.handler?.('conv_a'))
    await waitFor(() => expect(joins).toHaveLength(2))

    saved.after = [message('msg_3')]
    await act(async () => joins[1]!.ends(true))
    await waitFor(() => expect(result.current.messages.map((one) => one.id)).toEqual(['msg_1', 'msg_2', 'msg_3']))
    expect(calls.chatAfter).toHaveBeenLastCalledWith('p', 'conv_a', 'msg_2')
    expect(result.current.messages[0]).toBe(before[0])
    expect(result.current.messages[1]).toBe(before[1])
  })

  it('leave rewind working: the head names the turns, and rewinding forks', async () => {
    const { result } = renderHook(() => useChat('conv_a'))
    await waitFor(() => expect(result.current.profileId).toBe('claude'))
    act(() => result.current.say('hello'))
    await act(async () => ending(end))
    await waitFor(() => expect(result.current.rewindable).toEqual(['turn_1']))
    act(() => result.current.rewind('turn_1'))
    await waitFor(() => expect(shell.show).toHaveBeenCalledWith('chat', { id: 'conv_fork' }))
    expect(calls.chatRewind).toHaveBeenCalledWith('p', 'conv_a', 'turn_1')
  })
})
