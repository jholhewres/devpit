import { describe, expect, it } from 'vitest'

import { afterSaid, modelName, runningIn, type Subagents } from './subagents'

const id = 'a82d987eefe13ba95'
const started = JSON.stringify({ id, kind: 'general-purpose' })
const launched = JSON.stringify({
  id,
  name: 'probe child',
  model: 'claude-haiku-4-5-20251001',
  ended: false,
})
const stopped = JSON.stringify({ id, ended: true })

const through = (details: string[]): Subagents =>
  details.reduce<Subagents>((held, detail) => afterSaid(held, 'leaf_1', detail), {})

describe('the subagents a terminal agent started', () => {
  it('opens a row on start and names it when the call returns', () => {
    const rows = runningIn(through([started, launched]), ['leaf_1'])
    expect(rows).toEqual([
      { id, name: 'probe child', kind: 'general-purpose', model: 'claude-haiku-4-5-20251001', ended: false },
    ])
  })

  it('ends it on the first stop, and the repeats change nothing', () => {
    const once = through([started, launched, stopped])
    expect(runningIn(once, ['leaf_1'])).toEqual([])
    expect(afterSaid(once, 'leaf_1', stopped)).toBe(once)
  })

  it('does not bring an ended one back on a late launch report', () => {
    expect(runningIn(through([started, stopped, launched]), ['leaf_1'])).toEqual([])
  })

  it('keeps each pane to its own subagents', () => {
    expect(runningIn(through([started]), ['leaf_2'])).toEqual([])
  })

  it('ignores a detail it cannot read', () => {
    const held = through([started])
    for (const detail of [null, 'not json', '[]', '{"name":"no id"}']) {
      expect(afterSaid(held, 'leaf_1', detail)).toBe(held)
    }
  })

  it('shortens a model name to what fits the column', () => {
    expect(modelName('claude-haiku-4-5-20251001')).toBe('haiku-4-5')
    expect(modelName('opus')).toBe('opus')
  })
})
