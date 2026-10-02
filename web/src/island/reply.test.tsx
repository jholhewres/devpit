import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { IslandSession } from '../gen/bindings'
import { Reply } from './Reply'

const replied = vi.fn(async (..._args: unknown[]) => null)
const typing = vi.fn(async (_on: boolean) => null)
vi.mock('../shell/live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    islandReply: (...args: unknown[]) => replied(...args),
    islandTyping: (on: boolean) => typing(on),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

const inPane = { sessionId: 's1', paneId: 'leaf_1' } as IslandSession

describe('replying from the island', () => {
  it('takes the keyboard only while the field has it, and sends as the person', async () => {
    render(<Reply session={inPane} />)
    const field = screen.getByLabelText('Reply to this session, as you')
    fireEvent.focus(field)
    await waitFor(() => expect(typing).toHaveBeenCalledWith(true))
    fireEvent.change(field, { target: { value: 'yes, go on' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    await waitFor(() => expect(replied).toHaveBeenCalledWith('s1', 'yes, go on'))
    await waitFor(() => screen.getByText('Sent, as you.'))
    fireEvent.blur(field)
    await waitFor(() => expect(typing).toHaveBeenLastCalledWith(false))
  })

  it('has nothing to type into for a session in no terminal', () => {
    const { container } = render(<Reply session={{ sessionId: 'c1', paneId: null } as IslandSession} />)
    expect(container.innerHTML).toBe('')
  })
})
