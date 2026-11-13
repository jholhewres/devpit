import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Message, Part } from '../gen/bindings'
import { ActivityContext, type Activity } from './activity'
import { Turn } from './Turn'

vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))
vi.mock('./live', () => ({ ask: async () => ({ data: null, error: null }), commands: {} }))

afterEach(cleanup)

const text = (said: string): Part => ({ kind: 'text', text: said, parent: null })
const call = (id: string, command: string, ok = true): Part[] => [
  { kind: 'tool_call', id, name: 'Bash', input: JSON.stringify({ command }), state: ok ? 'ok' : 'failed', parent: null },
  { kind: 'tool_result', call_id: id, output: ok ? 'fine' : 'exit 1', is_error: !ok, parent: null },
]
const answer = (parts: Part[], streaming = false): Message => ({
  id: 'm1', turnId: 't1', role: 'assistant', parts, createdAt: 1_793_800_000, streaming,
  ended: streaming ? null : { at: 1_793_800_030, stopReason: 'end_turn', isError: false, error: null, unheard: [] },
})

const drawn = (activity: Activity, message: Message, steered = false): HTMLElement =>
  render(
    <ActivityContext.Provider value={activity}>
      <Turn message={message} steered={steered} />
    </ActivityContext.Provider>,
  ).container

const TURN = [text('First I look.'), ...call('b1', 'ls src'), text('Done: it builds.')]

describe('the activity a turn draws', () => {
  it('in Zen, only the final answer', () => {
    drawn('zen', answer(TURN))
    expect(screen.getByText('Done: it builds.')).toBeTruthy()
    expect(screen.queryByText('First I look.')).toBeNull()
    expect(screen.queryByText(/Worked/)).toBeNull()
  })

  it('in Zen, the replies of a turn somebody steered', () => {
    drawn('zen', answer(TURN), true)
    expect(screen.getByText('First I look.')).toBeTruthy()
    expect(screen.getByText('Done: it builds.')).toBeTruthy()
  })

  it('hiding all, the replies without the work', () => {
    const shown = drawn('none', answer(TURN, true))
    expect(screen.getByText('First I look.')).toBeTruthy()
    expect(shown.querySelector('.acts')).toBeNull()
  })

  it('hiding what succeeded, only the call that failed', () => {
    drawn('failed', answer([...call('b1', 'ls src'), ...call('b2', 'cat nope', false)], true))
    expect(screen.queryByText('ls src')).toBeNull()
    expect(screen.getAllByText('cat nope').length).toBeGreaterThan(0)
  })

  it('showing all, every row', () => {
    drawn('all', answer([...call('b1', 'ls src'), ...call('b2', 'cat nope', false)], true))
    expect(screen.getAllByText('ls src').length).toBeGreaterThan(0)
  })
})
