import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Message, Part } from '../gen/bindings'
import { ComposerStatus } from './ComposerStatus'

afterEach(cleanup)

const stopped = vi.fn()
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: { chatStopTask: (conversation: string, task: string) => (stopped(conversation, task), true) },
}))

const answer = (parts: Part[]): Message =>
  ({ id: 'm', turnId: 't', role: 'assistant', createdAt: 0, streaming: true, parts }) as Message

const started: Part = {
  kind: 'task', task_id: 'bxk020dqw', call_id: 'c', task_kind: 'local_bash',
  description: 'sleep 60 && echo late', status: 'started', summary: null,
}

describe('work left running, above the composer', () => {
  it('lists a running command and stops that one task', async () => {
    render(<ComposerStatus conversationId="conv_1" messages={[answer([started])]} />)
    expect(screen.getByText('sleep 60 && echo late')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Stop sleep 60 && echo late' }))
    await waitFor(() => expect(stopped).toHaveBeenCalledWith('conv_1', 'bxk020dqw'))
  })

  it('drops the row once the CLI says it stopped', () => {
    // The ending the probe measured: killed, then stopped.
    const ended: Part = { ...(started as Extract<Part, { kind: 'task' }>), status: 'stopped' }
    const { container } = render(<ComposerStatus conversationId="c" messages={[answer([started, ended])]} />)
    expect(container.firstChild).toBeNull()
  })
})

describe('the checklist, above the composer', () => {
  const create = (id: string, subject: string): Part[] => [
    { kind: 'tool_call', id: `c${id}`, name: 'TaskCreate', input: JSON.stringify({ subject }), state: 'ok', parent: null },
    { kind: 'tool_result', call_id: `c${id}`, output: JSON.stringify({ task: { id, subject } }), is_error: false, parent: null },
  ]

  it('counts what is done, and marks the item the last update changed', () => {
    const parts: Part[] = [
      ...create('1', 'Read notes.txt'),
      ...create('2', 'Edit notes.txt'),
      { kind: 'tool_call', id: 'u', name: 'TaskUpdate', input: JSON.stringify({ taskId: '1', status: 'completed' }), state: 'ok', parent: null },
    ]
    const { container } = render(<ComposerStatus conversationId="c" messages={[answer(parts)]} />)
    expect(screen.getByText('1 of 2 done')).toBeTruthy()
    expect(container.querySelector('[data-changed]')?.textContent).toContain('Read notes.txt')
  })

  it('reads only the newest answer', () => {
    const old = { ...answer(create('1', 'from an old turn')), id: 'old' }
    const { container } = render(<ComposerStatus conversationId="c" messages={[old, answer([])]} />)
    expect(container.firstChild).toBeNull()
  })
})
