import { describe, expect, it } from 'vitest'

import type { Part } from '../gen/bindings'
import { elapsed, running, tasksOf } from './backgroundTasks'
import { checklistOf, done } from './checklist'

const task = (id: string, status: string, over: Partial<Extract<Part, { kind: 'task' }>> = {}): Part => ({
  kind: 'task',
  task_id: id,
  call_id: null,
  task_kind: null,
  description: null,
  status,
  summary: null,
  ...over,
})

describe('the work left running', () => {
  it('keeps one row per task at its latest state', () => {
    // The order the recorded turn sent them in: started, then ended.
    const tasks = tasksOf([
      task('b85', 'started', { task_kind: 'local_bash', description: 'sleep 4 && echo background-done' }),
      task('b85', 'completed', { summary: 'Background command completed (exit code 0)' }),
    ])
    expect(tasks).toHaveLength(1)
    expect(tasks[0]).toMatchObject({
      kind: 'local_bash',
      description: 'sleep 4 && echo background-done',
      status: 'completed',
      summary: 'Background command completed (exit code 0)',
    })
    expect(running(tasks[0]!)).toBe(false)
  })

  it('treats started and progress as still going', () => {
    expect(running(tasksOf([task('a', 'started')])[0]!)).toBe(true)
    expect(running(tasksOf([task('a', 'started'), task('a', 'running')])[0]!)).toBe(true)
  })

  it('says how long in the composer’s words', () => {
    expect(elapsed(0, 42_000)).toBe('42s')
    expect(elapsed(0, 125_000)).toBe('2m 5s')
  })
})

const call = (id: string, name: string, input: object): Part => ({
  kind: 'tool_call', id, name, input: JSON.stringify(input), state: 'ok', parent: null,
})
const result = (id: string, output: object): Part => ({
  kind: 'tool_result', call_id: id, output: JSON.stringify(output), is_error: false, parent: null,
})

describe('the checklist', () => {
  /* The calls and results as Claude Code 2.1.270 sent them. */
  const created = [
    call('c1', 'TaskCreate', { subject: 'Read notes.txt', description: '…' }),
    result('c1', { task: { id: '1', subject: 'Read notes.txt' } }),
    call('c2', 'TaskCreate', { subject: 'Edit notes.txt', description: '…' }),
    result('c2', { task: { id: '2', subject: 'Edit notes.txt' } }),
  ]

  it('is built from the creates, each pending', () => {
    const list = checklistOf(created)
    expect(list.items.map((item) => [item.id, item.subject, item.status])).toEqual([
      ['1', 'Read notes.txt', 'pending'],
      ['2', 'Edit notes.txt', 'pending'],
    ])
  })

  it('follows each update, and says which item the last one changed', () => {
    const list = checklistOf([...created, call('u1', 'TaskUpdate', { taskId: '2', status: 'completed' })])
    expect(list.items.filter(done).map((item) => item.id)).toEqual(['2'])
    expect(list.changed).toEqual(['2'])
  })

  it('ignores an update for an item it never saw, and one that changes nothing', () => {
    const list = checklistOf([...created, call('u1', 'TaskUpdate', { taskId: '9', status: 'completed' }), call('u2', 'TaskUpdate', { taskId: '1', status: 'pending' })])
    expect(list.changed).toEqual([])
    expect(list.items.some(done)).toBe(false)
  })

  it('leaves out a subagent’s own checklist', () => {
    const theirs: Part = { ...(call('c9', 'TaskCreate', { subject: 'theirs' }) as Extract<Part, { kind: 'tool_call' }>), parent: 'toolu_agent' }
    expect(checklistOf([theirs, result('c9', { task: { id: '9', subject: 'theirs' } })]).items).toHaveLength(0)
  })
})
