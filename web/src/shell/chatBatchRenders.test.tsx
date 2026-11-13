import { act, cleanup, render, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Frame, Message } from '../gen/bindings'
import { ChatPane } from './ChatPane'

/*
 * A text batch of a streamed answer, through the real pane and the real chat,
 * from the transport in: what is drawn again for it.
 */

const drawn = new Map<string, number>()
const count = (what: string): null => (drawn.set(what, (drawn.get(what) ?? 0) + 1), null)

const history: Message[] = Array.from(
  { length: 40 },
  (_, at) =>
    ({
      id: `m${at}`,
      turnId: at % 2 ? `t${at}` : null,
      role: at % 2 ? 'assistant' : 'user',
      parts: [{ kind: 'text', text: `Message ${at}.`, parent: null }],
      createdAt: at,
      streaming: false,
    }) as Message,
)

/* The running turn the chat joined, and how its frames reach it. */
const joined: { onFrame: (frame: Frame) => void } = { onFrame: () => {} }
vi.mock('./chat', async (real) => ({
  ...(await real<typeof import('./chat')>()),
  rejoin: (_id: string, onFrame: (frame: Frame) => void) => (
    (joined.onFrame = onFrame), { running: new Promise<boolean>(() => {}), leave: () => {} }
  ),
  send: () => null,
}))
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: new Proxy(
    {
      chatHistory: (projectId: string, id: string) => ({ id, projectId, messages: history, older: false, profile: 'claude', rewindable: [], sessionId: null, costUsd: 0, cardId: null }),
      agentProfiles: () => [{ id: 'claude', path: '/bin/claude' }],
      chatPrompts: () => ({ prompts: [], total: 0 }),
    } as Record<string, unknown>,
    { get: (known, name: string) => known[name] ?? (() => null) },
  ),
}))
vi.mock('./window', async (real) => ({
  ...(await real<typeof import('./window')>()),
  onChatWoke: () => () => {},
  onPermissionAsked: () => () => {},
  onPermissionSettled: () => () => {},
}))
const nothing = (): void => {}
const shell = {
  active: { id: 'conv_1' },
  project: { id: 'p1', name: 'devpit', rootPath: '/w', orchestrator: null },
  rename: nothing,
  drafted: nothing,
  finished: nothing,
  show: nothing,
}
vi.mock('./useShell', () => ({ useShell: () => shell }))
vi.mock('./shellStore', async (real) => ({
  ...(await real<typeof import('./shellStore')>()),
  useShellPick: (pick: (from: typeof shell) => unknown) => pick(shell),
}))

/* Counted where each draws: the pane reads its chat, the corner is the pane's, the composer draws its footer, a turn its apps or its bubble. */
vi.mock('./useChat', async (real) => {
  const chat = await real<typeof import('./useChat')>()
  return { ...chat, useChat: (id: string) => (count('pane'), chat.useChat(id)) }
})
vi.mock('./PaneCorner', () => ({ PaneCorner: () => count('corner') }))
vi.mock('./ChatWhere', () => ({ ChatWhere: () => count('composer') }))
vi.mock('./TurnApps', () => ({ TurnApps: ({ messageId }: { messageId: string }) => count(messageId) }))
vi.mock('./Said', () => ({ Said: ({ message }: { message: Message }) => count(message.id) }))
vi.mock('./TurnDelegations', () => ({ TurnDelegations: () => null }))
vi.mock('./useSlash', () => ({ useSlash: () => ({ keyDown: () => false, ran: () => false, closeOwn: nothing }) }))
vi.mock('./useStop', () => ({ useStop: () => false }))
vi.mock('./Asked', () => ({ Asked: () => null }))
vi.mock('./Chips', () => ({ Chips: () => null }))
vi.mock('./ComposerStatus', () => ({ ComposerStatus: () => null }))
vi.mock('./ComposerTools', () => ({ ComposerTools: () => null }))
vi.mock('./SkillPills', () => ({ SkillPills: () => null }))
vi.mock('./SlashMenu', () => ({ SlashMenu: () => null }))
vi.mock('./OwnCommand', () => ({ OwnCommand: () => null }))
vi.mock('./DropTarget', () => ({ DropTarget: () => null }))

afterEach(() => {
  cleanup()
  drawn.clear()
})

const text = (piece: string): Frame => ({ type: 'part', message_id: 'live', part: { kind: 'text', text: piece, parent: null } }) as Frame
/* Past the batch's own wait, so what was pushed is applied. */
const settle = (): Promise<void> => act(() => new Promise<void>((done) => setTimeout(done, 150)))

describe('a text batch of an answer streaming into the chat', () => {
  it('draws the live turn again, and not the pane, its corner, the composer or any other turn', async () => {
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    await waitFor(() => expect(drawn.get('m39')).toBeGreaterThan(0))
    act(() => joined.onFrame({ type: 'opened', message: { id: 'live', turnId: 'tl', role: 'assistant', parts: [], createdAt: 99, streaming: true } as Message }))
    await settle()
    expect(drawn.get('live')).toBeGreaterThan(0)
    drawn.clear()

    for (const piece of ['The answer ', 'goes on ', 'and on.']) {
      act(() => joined.onFrame(text(piece)))
      await settle()
    }

    expect(drawn.get('live')).toBe(3)
    expect([...drawn.keys()].filter((what) => what !== 'live')).toEqual([])
  }, 20_000)
})
