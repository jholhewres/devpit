import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { ChatPane } from './ChatPane'
import { drafted, type Strip } from './strip'

afterEach(cleanup)

const said = vi.fn()
const taken = vi.fn()

const chat = {
  messages: [],
  rewindable: [],
  files: [],
  skills: [],
  previews: {},
  asked: [],
  error: null,
  sending: false,
  profileId: 'prof_1',
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
vi.mock('./useSlash', () => ({ useSlash: () => ({ keyDown: () => false }) }))
vi.mock('./useStop', () => ({ useStop: () => false }))
vi.mock('./useShell', () => ({
  useShell: () => ({ active: { id: 'conv_1' }, project: { id: 'p1', name: 'devpit' }, rename: vi.fn(), drafted: taken }),
}))
vi.mock('./Asked', () => ({ Asked: () => null }))
vi.mock('./Chips', () => ({ Chips: () => null }))
vi.mock('./ChatWhere', () => ({ ChatWhere: () => null }))
vi.mock('./ComposerStatus', () => ({ ComposerStatus: () => null }))
vi.mock('./CopySession', () => ({ CopySession: () => null }))
vi.mock('./DropTarget', () => ({ DropTarget: () => null }))
vi.mock('./PaneCorner', () => ({ PaneCorner: ({ children }: { children?: React.ReactNode }) => <>{children}</> }))
vi.mock('./Turn', () => ({ Turn: () => null }))
vi.mock('./SkillPills', () => ({ SkillPills: () => null }))
vi.mock('./SlashMenu', () => ({ SlashMenu: () => null }))

const draft = '# Wire the board\n\nAll of it.\n\nPinned files:\n- /w/notes.md'

describe('a chat opened from a card', () => {
  it('has the card in its composer, and sends nothing', () => {
    render(<ChatPane tab={{ id: 'conv_1', kind: 'chat', draft }} />)
    expect((screen.getByPlaceholderText(/^Do anything…/) as HTMLTextAreaElement).value).toBe(draft)
    expect(taken).toHaveBeenCalledWith('conv_1')
    expect(said).not.toHaveBeenCalled()
    // Its corner names the card it is about.
    expect(screen.getByText('Wire the board')).toBeTruthy()
  })

  it('stops carrying the draft once it is in the composer', () => {
    const strip: Strip = { open: [{ id: 'conv_1', kind: 'chat', draft }], active: 'conv_1' }
    expect(drafted(strip, 'conv_1').open[0]).toEqual({ id: 'conv_1', kind: 'chat' })
  })
})
