import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Message, Part } from '../gen/bindings'
import { acts } from './acts'
import { Acts } from './Acts'
import { Turn } from './Turn'

afterEach(cleanup)

vi.mock('./useShell', () => ({ useShell: () => ({ show: vi.fn() }) }))

/* The shape Claude Code 2.1.270 sent in the recorded turn: an `Agent` call that
   answers at once, work carrying `parent`, and a background task that ends it. */
const AGENT = 'toolu_agent'
const turn = (taskStatus: string | null): Part[] => [
  { kind: 'tool_call', id: AGENT, name: 'Agent', input: JSON.stringify({ description: 'Read notes.txt and count lines', prompt: 'x' }), state: 'ok', parent: null },
  { kind: 'tool_result', call_id: AGENT, output: 'Async agent launched', is_error: false, parent: null },
  { kind: 'tool_call', id: 'toolu_read', name: 'Read', input: JSON.stringify({ file_path: '/work/demo/notes.txt' }), state: 'ok', parent: AGENT },
  { kind: 'tool_result', call_id: 'toolu_read', output: '3 lines', is_error: false, parent: AGENT },
  { kind: 'text', text: 'notes.txt has 3 lines', parent: AGENT },
  ...(taskStatus
    ? [{ kind: 'task', task_id: 'a1', call_id: AGENT, task_kind: 'local_agent', description: 'Read notes.txt and count lines', status: taskStatus, summary: null } as Part]
    : []),
]

describe('what a subagent did', () => {
  it('sits under the call that started it, not beside it', () => {
    const rows = acts(turn('completed'))
    expect(rows.map((row) => row.name)).toEqual(['Agent'])
    expect(rows[0]!.kind).toBe('agent')
    expect(rows[0]!.children.map((row) => row.name)).toEqual(['Read'])
  })

  it('is still running until its task says it finished, though the call already answered', () => {
    expect(acts(turn('started'))[0]!.done).toBe(false)
    expect(acts(turn('running'))[0]!.done).toBe(false)
    expect(acts(turn('completed'))[0]!.done).toBe(true)
  })

  it('folds its work until opened, and is named by what it was asked to do', () => {
    render(<Acts parts={turn('completed')} live />)
    expect(screen.getByText('Read notes.txt and count lines')).toBeTruthy()
    expect(screen.queryByText('notes.txt')).toBeNull()
    fireEvent.click(screen.getByText('Read notes.txt and count lines'))
    expect(screen.getByText('notes.txt')).toBeTruthy()
  })

  it("is not the turn's answer", () => {
    const message = { id: 'm', turnId: 't', role: 'assistant', createdAt: 0, streaming: false, parts: [...turn('completed'), { kind: 'text', text: 'Done: 3 lines.', parent: null }] } as unknown as Message
    render(<Turn message={message} />)
    expect(screen.getByText('Done: 3 lines.')).toBeTruthy()
    expect(screen.queryByText('notes.txt has 3 lines')).toBeNull()
  })
})
