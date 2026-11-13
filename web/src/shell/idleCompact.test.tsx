import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { IdleCompact as Rule, Message } from '../gen/bindings'
import { appliedAll } from './frames'
import { IdleCompact } from './IdleCompact'
import { Turn } from './Turn'

let stored: Rule = { when: 'never', tokens: 200000 }
const write = vi.fn(async (_profile: string, rule: Rule) => (stored = rule))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    idleCompactRead: async () => stored,
    idleCompactWrite: (profile: string, rule: Rule) => write(profile, rule),
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ show: vi.fn() }) }))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  stored = { when: 'never', tokens: 200000 }
})

const note: Message = {
  id: 'n',
  turnId: null,
  role: 'system',
  createdAt: 0,
  streaming: false,
  ended: null,
  parts: [{ kind: 'compacted', tokens: 212000 }],
}

describe('compacting an idle conversation', () => {
  it('is never unless chosen, and takes when and past how many tokens', async () => {
    render(<IdleCompact profileId="claude" />)
    await waitFor(() => expect(screen.getByText('Never').getAttribute('aria-selected')).toBe('true'))
    fireEvent.click(screen.getByText('Just before expiry'))
    await waitFor(() => expect(write).toHaveBeenLastCalledWith('claude', { when: 'expiry', tokens: 200000 }))
    const tokens = await screen.findByLabelText('Compact past this many tokens')
    fireEvent.change(tokens, { target: { value: '150000' } })
    fireEvent.blur(tokens)
    await waitFor(() => expect(write).toHaveBeenLastCalledWith('claude', { when: 'expiry', tokens: 150000 }))
  })

  it('leaves a quiet note in the thread', () => {
    render(<Turn message={note} />)
    expect(screen.getByText('Compacted at 212k tokens')).toBeTruthy()
  })

  it('shows the note before the answer still being written, where the transcript keeps it', () => {
    const asked: Message = { ...note, id: 'u', turnId: 't', role: 'user', parts: [] }
    const answer: Message = { ...note, id: 'a', turnId: 't', role: 'assistant', parts: [], streaming: true }
    const after = appliedAll([asked, answer], [{ type: 'opened', message: note }])
    expect(after.map((one) => one.id)).toEqual(['u', 'n', 'a'])
  })
})
