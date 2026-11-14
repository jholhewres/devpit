import { cleanup, fireEvent, render, renderHook, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { ConversationMarks, Thread } from '../gen/bindings'
import { AboutSettings } from './AboutSettings'
import { ArchiveSettings } from './ArchiveSettings'
import { useConversations } from './useConversations'

const thread = (id: string, title: string, marks: Partial<ConversationMarks> = {}): Thread =>
  ({ id, title, profile: 'claude', model: null, costUsd: 0, lastAt: 1_800_000_000, cwd: null, marks }) as Thread

const listed = [thread('c1', 'Fix the parser'), thread('c2', 'Old spike', { archived: true, name: 'Spike on caching' })]
const marked = vi.fn()
const opened = vi.fn()
const reload = vi.fn()

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    chatList: async () => ({ conversations: listed }),
    chatMark: async (_p: string, id: string, marks: ConversationMarks) => (marked(id, marks), marks),
    agentProfiles: async () => [],
    appInfo: async () => ({ version: '0.1.50', platform: 'linux', statePath: '/x', dev: false }),
    urlOpen: async (url: string) => (opened(url), { opened: true }),
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'p1', name: 'devpit' }, reload }) }))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

describe('an archived conversation', () => {
  it('leaves the conversation lists', async () => {
    const { result } = renderHook(() => useConversations('p1', vi.fn(), [], vi.fn()))
    await waitFor(() => expect(result.current.threads.map((one) => one.id)).toEqual(['c1']))
  })

  it('is listed in Settings → Archive, and Restore puts it back', async () => {
    render(<ArchiveSettings />)
    await waitFor(() => expect(screen.getByText('Spike on caching')).toBeTruthy())
    expect(screen.queryByText('Fix the parser')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Restore' }))
    await waitFor(() => expect(screen.queryByText('Spike on caching')).toBeNull())
    expect(marked).toHaveBeenCalledWith('c2', expect.objectContaining({ archived: false, name: 'Spike on caching' }))
    expect(reload).toHaveBeenCalled()
  })
})

describe('Settings → About', () => {
  it('shows the version and opens the privacy policy and the licences', async () => {
    render(<AboutSettings />)
    await waitFor(() => expect(screen.getByText('devpit 0.1.50 · linux')).toBeTruthy())
    for (const button of screen.getAllByRole('button', { name: 'Open' })) fireEvent.click(button)
    await waitFor(() => expect(opened).toHaveBeenCalledTimes(3))
    expect(opened.mock.calls.map(([url]) => url)).toEqual([
      'https://devpit.app/privacy',
      'https://github.com/jholhewres/devpit/blob/main/LICENSE',
      'https://github.com/jholhewres/devpit/blob/main/NOTICE',
    ])
  })
})
