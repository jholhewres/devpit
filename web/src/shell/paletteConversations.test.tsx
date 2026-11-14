import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { SessionHit } from '../gen/bindings'
import { Palette } from './Palette'

afterEach(cleanup)

const searched = vi.fn()
const shell = {
  show: vi.fn(),
  openChat: vi.fn(),
  projects: [
    { id: 'prj_1', name: 'devpit' },
    { id: 'prj_2', name: 'website' },
  ],
}
const hits: SessionHit[] = [
  { sessionId: 'aaa', projectId: 'prj_2', role: 'user', snippet: 'why does the [lexer] drop tabs', installation: '/home/me/.claude', conversationId: 'conv_9' },
]

vi.mock('./useShell', () => ({ useShell: () => shell }))
vi.mock('./paletteReach', () => ({
  useReachable: () => ({ panes: [], agents: [], sessions: [], cards: [], files: [], indexing: false, partial: false }),
}))
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: {
    sessionsSearch: (projectId: string | null, query: string) => {
      searched(projectId, query)
      return hits
    },
  },
}))

describe('the palette', () => {
  it('finds a conversation of another project by what was said, and opens it there', async () => {
    render(<Palette onClose={vi.fn()} />)
    fireEvent.change(screen.getByPlaceholderText(/conversations/), { target: { value: 'lexer' } })
    const row = (await screen.findByText('why does the lexer drop tabs', {}, { timeout: 2000 })).closest('button')!
    expect(searched).toHaveBeenCalledWith(null, 'lexer')
    expect(row.textContent).toContain('website')
    fireEvent.click(row)
    await waitFor(() => expect(shell.openChat).toHaveBeenCalledWith('prj_2', 'conv_9'))
  })
})
