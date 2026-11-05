import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Question } from '../gen/bindings'
import { useChat } from './useChat'

afterEach(cleanup)

/* What the window hears about questions, so a test can say it. */
const heard: { asked: ((question: Question) => void) | null; settled: ((id: string) => void) | null } = { asked: null, settled: null }
const picked = vi.fn()

vi.mock('./chat', async (actual) => ({
  ...(await actual<typeof import('./chat')>()),
  rejoin: () => ({ running: new Promise<boolean>(() => {}), leave: () => {} }),
}))
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    chatHistory: (projectId: string, id: string) => ({ id, projectId, messages: [], profile: 'claude', rewindable: [], sessionId: 'ses_1', costUsd: 0, cardId: null }),
    agentProfiles: () => [{ id: 'claude', path: '/bin/claude' }],
    permissionAskFromNow: () => null,
    permissionPick: (id: string, chosen: string[][]) => (picked(id, chosen), null),
  },
}))
vi.mock('./window', () => ({
  inTauri: () => true,
  onChatWoke: () => () => {},
  onPermissionAsked: (then: (question: Question) => void) => ((heard.asked = then), () => {}),
  onPermissionSettled: (then: (id: string) => void) => ((heard.settled = then), () => {}),
}))
const shell = { project: { id: 'p', rootPath: '/w', orchestrator: null }, show: vi.fn() }
vi.mock('./shellStore', () => ({ useShellPick: (pick: (from: typeof shell) => unknown) => pick(shell) }))

const bash: Question = { id: 'q1', sessionId: 'ses_1', tool: 'Bash', input: '{"command":"ls"}', cwd: '/w', touch: { kind: 'run', command: 'ls' } }
const colour: Question = {
  id: 'q2',
  sessionId: 'ses_1',
  tool: 'AskUserQuestion',
  input: '{}',
  cwd: '/w',
  touch: { kind: 'ask', questions: [{ header: null, question: 'Which colour?', options: ['Red', 'Green', 'Blue'], multi: false }] },
}

describe("a chat's questions", () => {
  it('go from the chat once the island answered them, a permission and a question alike', async () => {
    const { result } = renderHook(() => useChat('conv_a'))
    await waitFor(() => expect(result.current.session).toBe('ses_1'))
    act(() => {
      heard.asked?.(bash)
      heard.asked?.(colour)
    })
    expect(result.current.asked.map((one) => one.id)).toEqual(['q1', 'q2'])
    act(() => heard.settled?.('q2'))
    expect(result.current.asked.map((one) => one.id)).toEqual(['q1'])
    act(() => heard.settled?.('q1'))
    expect(result.current.asked).toEqual([])
  })

  it('send the choices picked to the question they answer', async () => {
    const { result } = renderHook(() => useChat('conv_a'))
    await waitFor(() => expect(result.current.session).toBe('ses_1'))
    act(() => heard.asked?.(colour))
    act(() => result.current.answer('q2', 'allow', [['Green']]))
    expect(result.current.asked).toEqual([])
    await waitFor(() => expect(picked).toHaveBeenCalledWith('q2', [['Green']]))
  })
})
