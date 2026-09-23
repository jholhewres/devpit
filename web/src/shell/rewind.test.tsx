import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { Turn } from './Turn'

const shell = { show: vi.fn() }
vi.mock('./useShell', () => ({ useShell: () => shell }))

afterEach(cleanup)

const answer: Message = {
  id: 'm2',
  turnId: 'turn_1',
  role: 'assistant',
  parts: [{ kind: 'text', text: 'alpha', parent: null }],
  createdAt: 1,
  streaming: false,
}

describe('going back to an earlier turn', () => {
  it('is offered on a finished turn a fork can start from', () => {
    const rewind = vi.fn()
    render(<Turn message={answer} onRewind={rewind} />)
    fireEvent.click(screen.getByLabelText('Go back to this turn'))
    expect(rewind).toHaveBeenCalledWith('turn_1')
  })

  /* A turn that ran before its place in the CLI's transcript was kept has
     nothing to fork at, so the control is not drawn rather than failing. */
  it('is not offered where no fork is possible', () => {
    render(<Turn message={answer} />)
    expect(screen.queryByLabelText('Go back to this turn')).toBeNull()
  })

  it('is not offered while the turn is still arriving', () => {
    render(<Turn message={{ ...answer, streaming: true }} onRewind={vi.fn()} />)
    expect(screen.queryByLabelText('Go back to this turn')).toBeNull()
  })

  it('says in the fork where it came from, and opens that conversation', () => {
    const note: Message = {
      id: 'm3',
      turnId: null,
      role: 'system',
      parts: [{ kind: 'rewound', from_conversation: 'conv_a', turn: 2 }],
      createdAt: 1,
      streaming: false,
    }
    render(<Turn message={note} />)
    expect(screen.getByText(/Went back to turn 2/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'the conversation it came from' }))
    expect(shell.show).toHaveBeenCalledWith('chat', { id: 'conv_a' })
  })
})
