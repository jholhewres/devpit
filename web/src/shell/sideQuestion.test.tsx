import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { chatMessages } from './chatMessages'
import { ChatPane } from './ChatPane'

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

const asideAsked = vi.fn(async (_project: string, _conversation: string, _question: string) => 'Because the cache was cold.')
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: new Proxy({} as Record<string, unknown>, {
    get: (_, name) => (name === 'chatAside' ? asideAsked : name === 'chatSlashCommands' ? async () => ['compact'] : async () => null),
  }),
}))

/* A turn is running: the chat is sending, its answer still streaming. */
const working = [
  { id: 'm1', turnId: 't1', role: 'user', parts: [{ kind: 'text', text: 'Fix the build', parent: null }], createdAt: 1, streaming: false },
  { id: 'm2', turnId: 't1', role: 'assistant', parts: [{ kind: 'text', text: 'Looking', parent: null }], createdAt: 2, streaming: true },
] as Message[]
const nothing = vi.fn()
const said = vi.fn()
const steered = vi.fn(async () => true)
const stopped = vi.fn()
const chat = {
  messages: chatMessages(working), rewindable: [], files: [], skills: [], previews: {}, asked: [], error: null, sending: true,
  profileId: 'prof_1', profiles: [], cost: 0, session: 'ses_1', card: null, older: false, folder: null,
  say: said, steer: steered, stop: stopped, answer: nothing, detach: nothing, attach: nothing, paste: nothing, setSkills: nothing, rewind: nothing, took: nothing, clearExtras: nothing,
}
vi.mock('./useChat', () => ({ useChat: () => chat }))
vi.mock('./useStop', () => ({ useStop: () => false }))
vi.mock('./useShell', () => ({
  useShell: () => ({ active: { id: 'conv_1' }, project: { id: 'p1', name: 'devpit' }, rename: nothing, drafted: nothing }),
}))
vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))
vi.mock('./TurnApps', () => ({ TurnApps: () => null }))
vi.mock('./TurnDelegations', () => ({ TurnDelegations: () => null }))
vi.mock('./Asked', () => ({ Asked: () => null }))
vi.mock('./Chips', () => ({ Chips: () => null }))
vi.mock('./ChatWhere', () => ({ ChatWhere: () => null }))
vi.mock('./ComposerStatus', () => ({ ComposerStatus: () => null }))
vi.mock('./ComposerTools', () => ({ ComposerTools: () => null }))
vi.mock('./CopySession', () => ({ CopySession: () => null }))
vi.mock('./DropTarget', () => ({ DropTarget: () => null }))
vi.mock('./PaneCorner', () => ({ PaneCorner: () => null }))
vi.mock('./SkillPills', () => ({ SkillPills: () => null }))

describe('/btw while a turn runs', () => {
  it('answers on the side, in a bubble of its own, without stopping, steering or queueing anything', async () => {
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    await act(async () => {})
    const field = screen.getByRole('textbox', { name: 'Message to the agent' })
    fireEvent.change(field, { target: { value: '/btw why is it slow?' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    await act(async () => {})

    expect(asideAsked).toHaveBeenCalledWith('p1', 'conv_1', 'why is it slow?')
    const bubble = screen.getByRole('region', { name: 'Side question' })
    expect(bubble.textContent).toContain('why is it slow?')
    expect(bubble.textContent).toContain('Because the cache was cold.')
    expect(said).not.toHaveBeenCalled()
    expect(steered).not.toHaveBeenCalled()
    expect(stopped).not.toHaveBeenCalled()
    expect(screen.queryByText(/queued/i)).toBeNull()
    expect((field as HTMLTextAreaElement).value).toBe('')
  })
})
