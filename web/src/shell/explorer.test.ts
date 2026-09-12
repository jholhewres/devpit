import { beforeEach, describe, expect, it } from 'vitest'

import { empty, remember, remembered } from './explorer'

describe('remembering the Explorer panel per project', () => {
  beforeEach(() => localStorage.clear())

  it('opens on the tree for a project it has not seen', () => {
    expect(remembered('p1')).toEqual(empty)
  })

  it('gives back what that project was showing', () => {
    remember('p1', { view: 'changes', mode: 'contents', query: 'todo' })
    expect(remembered('p1')).toEqual({ view: 'changes', mode: 'contents', query: 'todo' })
  })

  it('keeps projects apart', () => {
    remember('p1', { view: 'changes', mode: 'names', query: 'a' })
    remember('p2', { view: 'history', mode: 'names', query: 'b' })
    expect(remembered('p1').view).toBe('changes')
    expect(remembered('p2').view).toBe('history')
  })

  it('opens on the tree when there is no project', () => {
    expect(remembered(null)).toEqual(empty)
  })

  it('survives a value it cannot parse', () => {
    localStorage.setItem('devpit.explorer', 'not json')
    expect(remembered('p1')).toEqual(empty)
  })
})
