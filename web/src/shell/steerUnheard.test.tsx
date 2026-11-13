import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { chatMessages } from './chatMessages'
import { ChatThread } from './ChatThread'

vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))
afterEach(cleanup)

const said = (id: string, text: string, turnId: string | null): Message =>
  ({ id, turnId, role: 'user', parts: [{ kind: 'text', text, parent: null }], createdAt: 0, streaming: false }) as Message

const nothing = (): void => {}

function thread(unheard: string[], onResend: (text: string) => void) {
  const answer = {
    id: 'a1',
    turnId: 't1',
    role: 'assistant',
    parts: [{ kind: 'text', text: 'Done.', parent: null }],
    createdAt: 0,
    streaming: false,
    ended: { at: 1, stopReason: 'success', isError: false, error: null, unheard },
  } as Message
  const messages = [said('u1', 'fix the board', 't1'), said('u2', 'and the lane too', null), answer]
  return render(
    <ChatThread messages={chatMessages(messages)} box={{ current: null }} top={{ current: null }} older={false} stalled={false} onMore={nothing} sending={false} error={null} folder={null} rewindable={[]} onRewind={nothing} onEdit={nothing} onOpen={nothing} onRetry={nothing} onContinue={nothing} onResend={onResend} />,
  )
}

describe('a message steered in that the agent never took in', () => {
  it('says so under it, and sends it again on Resend', () => {
    const onResend = vi.fn()
    thread(['u2'], onResend)
    expect(screen.getAllByText(/didn.t reach the agent/i)).toHaveLength(1)
    fireEvent.click(screen.getByRole('button', { name: 'Resend' }))
    expect(onResend).toHaveBeenCalledWith('and the lane too')
    /* Sent again, it is not offered a second time. */
    expect(screen.queryByText(/didn.t reach the agent/i)).toBeNull()
  })

  it('says nothing when the agent took it in', () => {
    thread([], vi.fn())
    expect(screen.queryByText(/didn.t reach the agent/i)).toBeNull()
  })
})
