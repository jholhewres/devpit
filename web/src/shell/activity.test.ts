import { afterEach, describe, expect, it } from 'vitest'

import type { Act } from './acts'
import { activityOf, drawn, keepActivity, seen, shownRows } from './activity'
import type { Segment } from './turnFold'

afterEach(() => localStorage.clear())

const row = (id: string, done: boolean, failed: boolean): Act => ({
  id, kind: 'run', name: 'Bash', target: id, input: '', output: '', failed, done, children: [],
})

describe('the activity a conversation draws', () => {
  it('is kept per conversation, over the default, over all', () => {
    expect(activityOf('c1')).toBe('all')
    keepActivity(null, 'none')
    expect(activityOf('c1')).toBe('none')
    keepActivity('c1', 'zen')
    expect(activityOf('c1')).toBe('zen')
    expect(activityOf('c2')).toBe('none')
  })

  it('shows everything while Find is open', () => {
    expect(seen('zen', true)).toBe('all')
    expect(seen('zen', false)).toBe('zen')
  })

  it('hides the calls that succeeded, keeping the failed and the running', () => {
    const rows = [row('ok', true, false), row('bad', true, true), row('going', false, false)]
    expect(shownRows(rows, 'failed').map((one) => one.id)).toEqual(['bad', 'going'])
    expect(shownRows(rows, 'none')).toEqual([])
    expect(shownRows(rows, 'all')).toBe(rows)
  })
})

describe('what a turn keeps', () => {
  const turn: Segment[] = [
    { kind: 'prose', at: 0, text: 'Looking.' },
    { kind: 'work', at: 1, parts: [] },
    { kind: 'prose', at: 2, text: 'Done: it was the cache.' },
  ]
  const kept = (activity: Parameters<typeof drawn>[3], steered = false): string[] =>
    turn.filter((one, at) => drawn(one, at, 2, activity, steered)).map((one) => (one.kind === 'prose' ? one.text : one.kind))

  it('in Zen, only the final answer', () => {
    expect(kept('zen')).toEqual(['Done: it was the cache.'])
  })

  it('in Zen, the answers to a steer too', () => {
    expect(kept('zen', true)).toEqual(['Looking.', 'Done: it was the cache.'])
  })

  it('without the work when it is hidden', () => {
    expect(kept('none')).toEqual(['Looking.', 'Done: it was the cache.'])
    expect(kept('all')).toEqual(['Looking.', 'work', 'Done: it was the cache.'])
  })
})
