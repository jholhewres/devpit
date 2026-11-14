import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { chatMessages } from './chatMessages'
import { ChatPane } from './ChatPane'

afterEach(cleanup)

const said = vi.fn()
const taken = vi.fn()

const chat = {
  messages: chatMessages([
    { id: 'm1', turnId: null, role: 'user', parts: [], createdAt: 0, streaming: false },
    { id: 'm2', turnId: null, role: 'assistant', parts: [], createdAt: 0, streaming: false },
  ] as Message[]),
  rewindable: [],
  files: [],
  skills: [],
  previews: {},
  asked: [],
  error: 'this conversation already has a turn running',
  sending: false,
  profileId: 'prof_1',
  profiles: [],
  cost: 0,
  session: null,
  card: { id: 'card_1', title: 'Wire the board', onBoard: false },
  say: said,
  stop: vi.fn(),
  answer: vi.fn(),
  detach: vi.fn(),
  attach: vi.fn(),
  paste: vi.fn(),
  setSkills: vi.fn(),
  rewind: vi.fn(),
}

vi.mock('./useChat', () => ({ useChat: () => chat }))
vi.mock('./useSlash', () => ({ useSlash: () => ({ keyDown: () => false, ran: () => false }) }))
vi.mock('./useStop', () => ({ useStop: () => false }))
vi.mock('./useShell', () => ({
  useShell: () => ({ active: { id: 'conv_1' }, project: { id: 'p1', name: 'devpit' }, rename: vi.fn(), threads: [], tellDoing: vi.fn(), mark: vi.fn(), reload: vi.fn(), drafted: taken }),
}))
vi.mock('./Asked', () => ({ Asked: () => null }))
vi.mock('./Chips', () => ({ Chips: () => null }))
vi.mock('./ChatWhere', () => ({ ChatWhere: () => null }))
vi.mock('./ComposerStatus', () => ({ ComposerStatus: () => null }))
vi.mock('./CopySession', () => ({ CopySession: () => null }))
vi.mock('./DropTarget', () => ({ DropTarget: () => null }))
vi.mock('./PaneCorner', () => ({ PaneCorner: ({ children }: { children?: React.ReactNode }) => <>{children}</> }))
vi.mock('./Turn', () => ({ Turn: () => <div data-testid="turn" /> }))
vi.mock('./SkillPills', () => ({ SkillPills: () => null }))
vi.mock('./SlashMenu', () => ({ SlashMenu: () => null }))

describe('a turn the chat could not run', () => {
  it('says so at the end of the thread, where the person is reading', () => {
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    const said = screen.getByRole('alert')
    expect(said.textContent).toContain('already has a turn running')
    const last = screen.getAllByTestId('turn').at(-1)!
    expect(last.compareDocumentPosition(said) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })
})

describe('a send the chat refused', () => {
  it('gives the words back to the composer', async () => {
    // Refused after the send left, as the app answers: on a later tick.
    said.mockImplementation((prompt: string, back?: (prompt: string) => void) => void Promise.resolve().then(() => back?.(prompt)))
    chat.error = null as unknown as string
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat' }} />)
    const field = screen.getByPlaceholderText(/^Do anything…/) as HTMLTextAreaElement
    fireEvent.change(field, { target: { value: 'ship it' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    expect(said).toHaveBeenCalled()
    await waitFor(() => expect(field.value).toBe('ship it'))
  })
})
