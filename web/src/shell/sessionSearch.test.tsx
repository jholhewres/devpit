import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Project, SessionHit } from '../gen/bindings'
import { SessionSearch } from './SessionSearch'
import { pieces } from './snippet'

afterEach(cleanup)

describe('a hit’s words', () => {
  it('marks the matched words and leaves the rest as text', () => {
    expect(pieces('why does the [lexer] drop [tabs]…')).toEqual([
      { text: 'why does the ', hit: false },
      { text: 'lexer', hit: true },
      { text: ' drop ', hit: false },
      { text: 'tabs', hit: true },
      { text: '…', hit: false },
    ])
  })

  it('is plain text when nothing is marked, and never markup', () => {
    expect(pieces('<script>alert(1)</script>')).toEqual([{ text: '<script>alert(1)</script>', hit: false }])
  })
})

const show = vi.fn()
const adopted = vi.fn()
const project = { id: 'prj_1', name: 'demos' } as unknown as Project
let hits: SessionHit[] = []
vi.mock('./useShell', () => ({ useShell: () => ({ project, show }) }))
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: {
    sessionsSearch: () => hits,
    cliInstallations: () => [{ directory: '/home/me/.claude', profiles: [], default: true }],
    agentProfiles: () => [{ id: 'claude', label: 'Claude Code', driver: 'claude', reach: 'runnable', mine: false }],
    chatAdopt: (projectId: string, sessionId: string, profileId: string) => {
      adopted(projectId, sessionId, profileId)
      return 'conv_new'
    },
  },
}))

describe('searching what was said', () => {
  it('opens the conversation devpit already holds for a hit', async () => {
    hits = [{ sessionId: 'aaa', role: 'user', snippet: 'the [lexer]', installation: '/home/me/.claude', conversationId: 'conv_1' }]
    render(<SessionSearch />)
    fireEvent.change(screen.getByLabelText('Search conversations'), { target: { value: 'lexer' } })
    fireEvent.click(await screen.findByText('lexer', {}, { timeout: 2000 }))
    await waitFor(() => expect(show).toHaveBeenCalledWith('chat', { id: 'conv_1' }))
    expect(adopted).not.toHaveBeenCalled()
  })

  it('takes in a session devpit never held, under a profile of its installation', async () => {
    hits = [{ sessionId: 'bbb', role: 'user', snippet: 'rename the [tokenizer]', installation: '/home/me/.claude', conversationId: null }]
    render(<SessionSearch />)
    fireEvent.change(screen.getByLabelText('Search conversations'), { target: { value: 'tokenizer' } })
    fireEvent.click(await screen.findByText('tokenizer', {}, { timeout: 2000 }))
    await waitFor(() => expect(adopted).toHaveBeenCalledWith('prj_1', 'bbb', 'claude'))
    expect(show).toHaveBeenCalledWith('chat', { id: 'conv_new' })
  })
})
