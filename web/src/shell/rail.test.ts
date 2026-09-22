import { describe, expect, it } from 'vitest'

import type { Project } from '../gen/bindings'
import { hueOf, initials, moved, ordered, sections } from './rail'

const project = (id: string, name = id, group: string | null = null): Project =>
  ({ id, name, group, rootPath: `/w/${name}` }) as unknown as Project

describe('the rail', () => {
  it('keeps the order the person gave it, whatever was opened last', () => {
    const byClock = [project('b'), project('a'), project('c')]
    expect(ordered(byClock, ['a', 'b', 'c']).map((one) => one.id)).toEqual(['a', 'b', 'c'])
  })

  it('puts a project nobody placed yet at the end, by name', () => {
    const list = [project('z', 'zeta'), project('x', 'alpha'), project('a')]
    expect(ordered(list, ['a']).map((one) => one.id)).toEqual(['a', 'x', 'z'])
  })

  it('moves an icon to where another one is, in either direction', () => {
    expect(moved(['a', 'b', 'c', 'd'], 'a', 'c')).toEqual(['b', 'c', 'a', 'd'])
    expect(moved(['a', 'b', 'c', 'd'], 'd', 'b')).toEqual(['a', 'd', 'b', 'c'])
    expect(moved(['a', 'b'], 'a', 'a')).toEqual(['a', 'b'])
  })

  it('names a project by one or two letters', () => {
    expect(initials('devpit')).toBe('DE')
    expect(initials('devpit-app')).toBe('DA')
    expect(initials('my project')).toBe('MP')
    expect(initials('x')).toBe('X')
  })

  it('gives each project a hue of its own that does not change between runs', () => {
    expect(hueOf('prj_01M2S1D4YD9KYTAZCGECKP22DP')).toBe(hueOf('prj_01M2S1D4YD9KYTAZCGECKP22DP'))
    expect(hueOf('prj_a')).not.toBe(hueOf('prj_b'))
    expect(hueOf('prj_a')).toBeGreaterThanOrEqual(0)
    expect(hueOf('prj_a')).toBeLessThan(360)
  })
})

describe('groups in the rail', () => {
  it('puts the ungrouped first, then each group where its first project stands', () => {
    const list = [project('a', 'a', 'Work'), project('b'), project('c', 'c', 'Home'), project('d', 'd', 'Work')]
    expect(sections(list).map((one) => [one.group, one.projects.map((p) => p.id)])).toEqual([
      [null, ['b']],
      ['Work', ['a', 'd']],
      ['Home', ['c']],
    ])
  })

  it('has no empty section when every project is in a group', () => {
    expect(sections([project('a', 'a', 'Work')]).map((one) => one.group)).toEqual(['Work'])
  })
})
