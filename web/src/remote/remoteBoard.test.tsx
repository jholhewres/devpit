import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Board } from '../gen/bindings'
import { RemoteBoard } from './RemoteBoard'

afterEach(cleanup)

const board = {
  projectId: 'p1',
  columns: [{ id: 'c1', name: 'inbox', step: null }],
  cards: [{ id: 'k1', title: 'Fix login', columnId: 'c1', position: 0 }],
} as unknown as Board

describe('the Remote board', () => {
  it('says first what waits on you', () => {
    const onWaiting = vi.fn()
    render(<RemoteBoard board={board} project="p1" typing={false} waiting={2} onWaiting={onWaiting} send={vi.fn()} />)
    fireEvent.click(screen.getByText('2 waiting on you'))
    expect(onWaiting).toHaveBeenCalled()
    expect(screen.queryByText('Start a session')).toBeNull()
  })

  it('makes a card and starts a session on one, for a device that may type', () => {
    const send = vi.fn()
    render(<RemoteBoard board={board} project="p1" typing waiting={0} onWaiting={vi.fn()} send={send} />)
    fireEvent.change(screen.getByLabelText('New card'), { target: { value: 'Ship it' } })
    fireEvent.click(screen.getByText('Add'))
    expect(send).toHaveBeenCalledWith({ t: 'cardCreate', project: 'p1', title: 'Ship it' })
    fireEvent.click(screen.getByText('Start a session'))
    fireEvent.click(screen.getByText('Start'))
    expect(send).toHaveBeenCalledWith({ t: 'sessionStart', project: 'p1', card: 'k1', prompt: 'Work on this card: Fix login', anyway: false })
  })
})
