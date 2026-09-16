import { describe, expect, it } from 'vitest'

import type { Notice } from '../gen/bindings'
import { held, minutesIn, type HeadsDown } from './headsDown'

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
