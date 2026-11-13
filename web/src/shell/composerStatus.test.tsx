import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Message, Part } from '../gen/bindings'
import { chatMessages } from './chatMessages'
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

/* What the driver reads off `claude-2.1.270-subagent-edit-tasks-background.jsonl`
   (lines 17, 23, 39, 46 and its result): a subagent, a shell and a wakeup. */
const RECORDED: Part[] = [
  { kind: 'task', task_id: 'a309a238b7c41f1c0', call_id: 'toolu_01UpZEX2rYBKuUp13WR2gekX', task_kind: 'local_agent', description: 'Read notes.txt and count lines', status: 'started', summary: null },
  { kind: 'task', task_id: 'a309a238b7c41f1c0', call_id: 'toolu_01UpZEX2rYBKuUp13WR2gekX', task_kind: null, description: 'Reading notes.txt', status: 'running', summary: null, duration_ms: 2425 },
  { kind: 'task', task_id: 'b85lyx22g', call_id: 'toolu_01DkB1Vr2yBQmLb8S5HiobsC', task_kind: 'local_bash', description: 'sleep 4 && echo background-done', status: 'started', summary: null },
  {
    kind: 'tool_call', id: 'toolu_01QUv9EgKw3j31gWB6aV19Bc', name: 'ScheduleWakeup', state: 'running', parent: null,
    input: JSON.stringify({ delaySeconds: 300, reason: 'Waiting for agent and background command to complete', noop: true }),
  },
  {
    kind: 'tool_result', call_id: 'toolu_01QUv9EgKw3j31gWB6aV19Bc', is_error: false, parent: null,
    output: 'Next wakeup scheduled for 15:47:00 (in 257s). Nothing more to do this turn — the harness re-invokes you when the wakeup fires or a task-notification arrives.',
  },
]

describe('work left running, above the composer', () => {
  it('lists a running command and stops that one task', async () => {
    render(<ComposerStatus conversationId="conv_1" messages={chatMessages([answer([started])])} />)
    fireEvent.click(screen.getByRole('button', { name: '1 in background' }))
    expect(screen.getByText('sleep 60 && echo late')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Stop sleep 60 && echo late' }))
    await waitFor(() => expect(stopped).toHaveBeenCalledWith('conv_1', 'bxk020dqw'))
  })

  it('opens from one chip to the subagent, the shell and the wakeup a recorded turn left', () => {
    render(<ComposerStatus conversationId="c" messages={chatMessages([answer(RECORDED)])} />)
    expect(screen.queryByText('Reading notes.txt')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: '3 in background' }))
    const panel = screen.getByRole('region', { name: 'In the background' })
    expect(panel.textContent).toContain('SubagentReading notes.txt')
    expect(panel.textContent).toContain('Shellsleep 4 && echo background-done')
    expect(panel.textContent).toContain('WakeupWaiting for agent and background command to complete15:47:00')
    // Stop where the CLI can stop it: the tasks, not the wakeup.
    expect(screen.getAllByRole('button', { name: /^Stop / })).toHaveLength(2)
  })

  it('drops the row once the CLI says it stopped', () => {
    // The ending the probe measured: killed, then stopped.
    const ended: Part = { ...(started as Extract<Part, { kind: 'task' }>), status: 'stopped' }
    const { container } = render(<ComposerStatus conversationId="c" messages={chatMessages([answer([started, ended])])} />)
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
    const { container } = render(<ComposerStatus conversationId="c" messages={chatMessages([answer(parts)])} />)
    expect(screen.getByText('1 of 2 done')).toBeTruthy()
    expect(container.querySelector('[data-changed]')?.textContent).toContain('Read notes.txt')
  })

  it('reads only the newest answer', () => {
    const old = { ...answer(create('1', 'from an old turn')), id: 'old' }
    const { container } = render(<ComposerStatus conversationId="c" messages={chatMessages([old, answer([])])} />)
    expect(container.firstChild).toBeNull()
  })
})
