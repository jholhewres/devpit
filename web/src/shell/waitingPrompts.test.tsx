import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { LiveSession } from '../gen/bindings'
import { inOrder } from './liveStatus'
import { WaitingPrompts } from './WaitingPrompts'

const answered = vi.fn()
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    orchestratorAnswer: (...args: unknown[]) => (answered(...args), null),
  },
}))

afterEach(() => {
  cleanup()
  answered.mockReset()
})

const session = (name: string, waiting: LiveSession['waiting'], status = 'idle'): LiveSession =>
  ({ name, status, kind: 'interactive', cwd: '/w', projectId: 'p', projectName: 'api', cardId: null, since: null, inDevpit: true, waiting }) as LiveSession

const asking = session('api-worker', {
  question: 'How do we split it?',
  options: [
    { label: 'One slice', hint: 'the spine first' },
    { label: 'All at once', hint: null },
  ],
  cursor: 0,
})

describe('the questions sessions are stopped on', () => {
  it('shows each one with its choices, and nothing when none waits', () => {
    const { container } = render(<WaitingPrompts profileId="claude" sessions={[session('quiet', null)]} onAnswered={() => {}} />)
    expect(container.textContent).toBe('')
    cleanup()
    render(<WaitingPrompts profileId="claude" sessions={[asking]} onAnswered={() => {}} />)
    expect(screen.getByText('How do we split it?')).toBeTruthy()
    expect(screen.getByText('All at once')).toBeTruthy()
  })

  it('answers with the choice and the whole question it was shown', async () => {
    const onAnswered = vi.fn()
    render(<WaitingPrompts profileId="claude" sessions={[asking]} onAnswered={onAnswered} />)
    fireEvent.click(screen.getByText('All at once'))
    await waitFor(() => expect(answered).toHaveBeenCalledWith('claude', 'api-worker', asking.waiting, 1))
    expect(onAnswered).toHaveBeenCalled()
  })

  it('takes no second pick while the first is on its way', async () => {
    let arrive: (value: unknown) => void = () => {}
    const slow = vi.fn(() => new Promise((resolve) => (arrive = resolve)))
    const live = await import('./live')
    const was = live.ask
    ;(live as { ask: unknown }).ask = slow
    render(<WaitingPrompts profileId="claude" sessions={[asking]} onAnswered={() => {}} />)
    fireEvent.click(screen.getByText('All at once'))
    expect((screen.getByText('One slice').closest('button') as HTMLButtonElement).disabled).toBe(true)
    fireEvent.click(screen.getByText('One slice'))
    expect(slow).toHaveBeenCalledTimes(1)
    arrive({ data: null, error: null })
    await waitFor(() => expect((screen.getByText('One slice').closest('button') as HTMLButtonElement).disabled).toBe(false))
    ;(live as { ask: unknown }).ask = was
  })

  it('dismisses with Esc', async () => {
    render(<WaitingPrompts profileId="claude" sessions={[asking]} onAnswered={() => {}} />)
    fireEvent.click(screen.getByText('Esc'))
    await waitFor(() => expect(answered).toHaveBeenCalledWith('claude', 'api-worker', asking.waiting, null))
  })

  it('puts a session waiting on the person above a busy one', () => {
    const ordered = inOrder([session('busy', null, 'busy'), asking, session('idle', null)])
    expect(ordered.map((one) => one.name)).toEqual(['api-worker', 'busy', 'idle'])
  })
})
