import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { chatMessages } from './chatMessages'
import { ChatPane } from './ChatPane'

afterEach(cleanup)

const messages = Array.from(
  { length: 500 },
  (_, at) =>
    ({
      id: `m${at}`,
      turnId: null,
      role: at % 2 ? 'assistant' : 'user',
      parts: [{ kind: 'text', text: `Message ${at}`, parent: null }],
      createdAt: at,
      streaming: false,
    }) as Message,
)

const nothing = vi.fn()
const chat = {
  messages: chatMessages(messages), rewindable: [], files: [], skills: [], previews: {}, asked: [], error: null, sending: false,
  profileId: 'prof_1', profiles: [], cost: 0, session: null, card: null, older: false, folder: null,
  say: nothing, stop: nothing, answer: nothing, detach: nothing, attach: nothing, paste: nothing, setSkills: nothing, rewind: nothing, took: nothing,
}

/* Counted where each is drawn: the pane reads its chat, the thread its window, a turn its acts. */
const drawn = { pane: 0, thread: 0, turns: 0 }
vi.mock('./useChat', () => ({ useChat: () => (drawn.pane++, chat) }))
vi.mock('./useWindow', async (real) => {
  const window = await real<typeof import('./useWindow')>()
  return { ...window, useWindow: (...args: Parameters<typeof window.useWindow>) => (drawn.thread++, window.useWindow(...args)) }
})
vi.mock('./TurnApps', () => ({ TurnApps: () => (drawn.turns++, null) }))
vi.mock('./useSlash', () => ({ useSlash: () => ({ keyDown: () => false, ran: () => false }) }))
vi.mock('./useStop', () => ({ useStop: () => false }))
vi.mock('./useShell', () => ({
  useShell: () => ({ active: { id: 'conv_1' }, project: { id: 'p1', name: 'devpit' }, rename: nothing, drafted: nothing }),
}))
vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))
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
vi.mock('./SlashMenu', () => ({ SlashMenu: () => null }))

describe('typing in the composer of a long conversation', () => {
  it('draws neither the pane, nor the thread, nor any turn again', () => {
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    const field = screen.getByPlaceholderText(/^Do anything…/) as HTMLTextAreaElement
    expect(drawn.turns).toBeGreaterThan(0)
    Object.assign(drawn, { pane: 0, thread: 0, turns: 0 })

    let text = ''
    for (let at = 0; at < 200; at += 1) {
      text += 'abcdefghij'[at % 10]
      fireEvent.change(field, { target: { value: text } })
    }

    expect(field.value).toHaveLength(200)
    expect(drawn).toEqual({ pane: 0, thread: 0, turns: 0 })
  })
})

describe('the composer read with a screen reader', () => {
  it('names its field, not only a placeholder that goes once typing starts', () => {
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    expect(screen.getByRole('textbox', { name: 'Message to the agent' })).toBeTruthy()
  })
})
