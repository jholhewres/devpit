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
const joins: { conversationId: string; onFrame: (frame: Frame) => void; leave: ReturnType<typeof vi.fn> }[] = []
const woke: { handler: ((conversationId: string) => void) | null } = { handler: null }
let ending: (end: TurnEnd) => void = () => {}

vi.mock('./chat', async (actual) => ({
  ...(await actual<typeof import('./chat')>()),
  rejoin: (conversationId: string, onFrame: (frame: Frame) => void) => {
    const leave = vi.fn()
    joins.push({ conversationId, onFrame, leave })
    return { running: new Promise<boolean>(() => {}), leave }
  },
  send: () => ({ end: new Promise<TurnEnd>((resolve) => (ending = resolve)) }),
}))
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    chatHistory: (projectId: string, id: string) => ({
      id,
      projectId,
      messages: [],
      profile: 'claude',
      rewindable: [],
      sessionId: null,
      costUsd: 0,
      cardId: null,
    }),
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
const shell = { project: { id: 'p', rootPath: '/w', orchestrator: null }, show: () => {} }
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
