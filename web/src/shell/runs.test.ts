import { describe, expect, it } from 'vitest'

import { asQuery } from './runs'

const none = { stepId: null, state: null, from: '', to: '' }

describe('the query the runs view sends', () => {
  it('narrows by nothing until asked', () => {
    expect(asQuery('prj', none, null)).toEqual({
      projectId: 'prj',
      stepId: null,
      state: null,
      since: null,
      until: null,
      after: null,
    })
  })

  /* Local days, and the `to` day included: a run late on the 1st is found by
     from = to = the 1st, wherever the machine is. */
  it('turns the date fields into a range that includes both days', () => {
    const query = asQuery('prj', { ...none, from: '2026-09-01', to: '2026-09-01' }, null)
    expect(query.since).toBe(new Date(2026, 8, 1).getTime() / 1000)
    expect(query.until).toBe(new Date(2026, 8, 2).getTime() / 1000)
  })

  it('carries the lane, the state and where the last page ended', () => {
    const after = { startedAt: 100, id: 'run_1' }
    const query = asQuery('prj', { ...none, stepId: 'step_1', state: 'lost' }, after)
    expect([query.stepId, query.state, query.after]).toEqual(['step_1', 'lost', after])
  })
})
