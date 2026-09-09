import { beforeEach, describe, expect, it } from 'vitest'

import { empty, remember, remembered } from './tabs'

describe('remembering tabs per project', () => {
  beforeEach(() => localStorage.clear())

  it('opens on nothing for a project it has not seen', () => {
    expect(remembered('p1')).toEqual(empty)
  })

  it('gives back what that project had open', () => {
    remember('p1', { open: ['board', 'term'], active: 'term' } as never)
    expect(remembered('p1')).toEqual({ open: ['board', 'term'], active: 'term' })
  })

  it('keeps projects apart', () => {
    remember('p1', { open: ['board'], active: 'board' } as never)
    remember('p2', { open: ['chat'], active: 'chat' } as never)
    expect(remembered('p1').open).toEqual(['board'])
    expect(remembered('p2').open).toEqual(['chat'])
  })

  it('opens on nothing when there is no project', () => {
    expect(remembered(null)).toEqual(empty)
  })

  it('survives a value it cannot parse', () => {
    localStorage.setItem('devpit.tabs', 'not json')
    expect(remembered('p1')).toEqual(empty)
  })
})
