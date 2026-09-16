import { describe, expect, it } from 'vitest'

import type { Notice } from '../gen/bindings'
import { held, minutesIn, nextThatNeedsYou, whatYouDid, type HeadsDown } from './headsDown'

const focus: HeadsDown = { projectId: 'prj_here', since: 1000 }

const notice = (over: Partial<Notice> = {}): Pick<Notice, 'projectId' | 'kind' | 'createdAt'> => ({
  projectId: 'prj_far',
  kind: 'run',
  createdAt: 2000,
  ...over,
})

/* Each branch is a decision somebody argued about, so each one is asserted
   rather than covered. Invert any single condition in `held` and one of these
   fails. */
describe('what a focus holds back', () => {
  it('holds what arrived from another project after it began', () => {
    expect(held(notice(), focus)).toBe(true)
  })

  it('holds nothing when there is no focus on', () => {
    expect(held(notice(), null)).toBe(false)
  })

  it('does not hold what was already there before it began', () => {
    expect(held(notice({ createdAt: 999 }), focus)).toBe(false)
    // The moment it began counts as inside it.
    expect(held(notice({ createdAt: 1000 }), focus)).toBe(true)
  })

  it('never holds this project back — being in it is the point', () => {
    expect(held(notice({ projectId: 'prj_here' }), focus)).toBe(false)
  })

  /* Waiting costs most exactly here, so it goes through the door from
     anywhere. */
  it('lets a step with no undo through, whichever project it is from', () => {
    expect(held(notice({ kind: 'irreversible' }), focus)).toBe(false)
    expect(held(notice({ kind: 'irreversible', projectId: 'prj_here' }), focus)).toBe(false)
  })

  /* Unknown is not "somewhere else": it comes from a card whose project could
     not be resolved, and a summary grouped by project would file it under
     nothing. */
  it('does not hold one whose project is unknown', () => {
    expect(held(notice({ projectId: null }), focus)).toBe(false)
  })

  /* Same reason: without a timestamp there is no telling whether it arrived
     during the focus, and what cannot be placed is not hidden. */
  it('does not hold one with no timestamp', () => {
    expect(held(notice({ createdAt: null }), focus)).toBe(false)
  })

  /* `since` crosses the contract as `number | null`: a float has values JSON
     cannot carry. A focus with no beginning holds nothing. */
  it('holds nothing when the focus has no beginning', () => {
    expect(held(notice(), { projectId: 'prj_here', since: null })).toBe(false)
    expect(minutesIn({ projectId: 'prj_here', since: null }, 9999)).toBe(0)
  })

  it('counts the focus in whole minutes from when it began', () => {
    expect(minutesIn(focus, 1000)).toBe(0)
    expect(minutesIn(focus, 1059)).toBe(0)
    expect(minutesIn(focus, 1060)).toBe(1)
    expect(minutesIn(focus, 1000 + 42 * 60)).toBe(42)
    // A clock that went backwards is not a negative focus.
    expect(minutesIn(focus, 900)).toBe(0)
  })
})

/* The bell's split, as a list rather than one notice at a time: this is what
   the panel shows and what the queue holds, from the same read. */
describe('splitting what the bell knows', () => {
  const list = [
    { id: 'a', projectId: 'prj_here', kind: 'run', createdAt: 2000 },
    { id: 'b', projectId: 'prj_far', kind: 'run', createdAt: 2000 },
    { id: 'c', projectId: 'prj_far', kind: 'irreversible', createdAt: 2000 },
    { id: 'd', projectId: 'prj_far', kind: 'run', createdAt: 500 },
    { id: 'e', projectId: null, kind: 'run', createdAt: 2000 },
  ]

  it('holds only what came from elsewhere after it began', () => {
    const waiting = list.filter((one) => held(one, focus)).map((one) => one.id)
    expect(waiting).toEqual(['b'])
  })

  it('holds nothing at all when no focus is on', () => {
    expect(list.filter((one) => held(one, null))).toEqual([])
  })
})

/* The order somebody stuck between five agents actually wants. */
describe('the next thing that needs you', () => {
  const one = (over: Partial<Notice>): Pick<Notice, 'id' | 'kind' | 'readAt' | 'createdAt'> => ({
    id: 'x',
    kind: 'run',
    readAt: null,
    createdAt: 1000,
    ...over,
  })

  it('answers nothing when there is nothing', () => {
    expect(nextThatNeedsYou([])).toBeNull()
  })

  it('puts an agent that is waiting before anything else', () => {
    const picked = nextThatNeedsYou([
      one({ id: 'run', kind: 'run' }),
      one({ id: 'due', kind: 'due' }),
      one({ id: 'agent', kind: 'agent' }),
      one({ id: 'undo', kind: 'irreversible' }),
    ])
    expect(picked?.id).toBe('agent')
  })

  it('puts a step with no undo second, before an ordinary run', () => {
    const picked = nextThatNeedsYou([one({ id: 'run' }), one({ id: 'undo', kind: 'irreversible' })])
    expect(picked?.id).toBe('undo')
  })

  it('prefers what nobody looked at, then the one waiting longest', () => {
    const picked = nextThatNeedsYou([
      one({ id: 'seen', readAt: 500 }),
      one({ id: 'newer', createdAt: 9000 }),
      one({ id: 'older', createdAt: 2000 }),
    ])
    expect(picked?.id).toBe('older')
  })

  /* A kind this build does not know sorts last rather than throwing: the set
     grows with whatever learns to notice something. */
  it('puts a kind it has never heard of last', () => {
    const picked = nextThatNeedsYou([one({ id: 'strange', kind: 'whatever' }), one({ id: 'run' })])
    expect(picked?.id).toBe('run')
  })
})

/* The inside half: what you did while the door was shut. */
describe('what you did', () => {
  const run = (state: string, costUsd: number | null = null) => ({ run: { state, costUsd } })

  it('counts nothing out of nothing', () => {
    expect(whatYouDid([])).toEqual({ ok: 0, failed: 0, costUsd: null, uncosted: 0 })
  })

  it('counts what ended, each way', () => {
    const said = whatYouDid([run('ok'), run('ok'), run('failed'), run('cancelled')])
    expect(said.ok).toBe(2)
    expect(said.failed).toBe(1)
  })

  /* A command step never records a cost. Folding it in as zero would make the
     total read as complete when it is a sum over agent turns only. */
  it('sums only what cost something, and says how much it could not see', () => {
    const said = whatYouDid([run('ok', 0.12), run('ok', 0.3), run('ok'), run('failed')])
    expect(said.costUsd).toBeCloseTo(0.42)
    expect(said.uncosted).toBe(2)
  })

  it('has no cost at all when nothing recorded one', () => {
    const said = whatYouDid([run('ok'), run('failed')])
    expect(said.costUsd).toBeNull()
    expect(said.uncosted).toBe(2)
  })
})
