import { describe, expect, it } from 'vitest'

import { ranked, score } from './search'

describe('how a match is ranked', () => {
  /* The last segment is what people type. */
  it('ranks the file name above the folder it is in', () => {
    expect(score('apps/desktop/src/main.rs', 'main')).toBeGreaterThan(
      score('main/other/thing.rs', 'main'),
    )
  })

  it('ranks a prefix of the name above a match inside it', () => {
    expect(score('src/main.rs', 'main')).toBeGreaterThan(score('src/domain.rs', 'main'))
  })

  it('gives nothing that does not match a score at all', () => {
    expect(score('src/main.rs', 'zzz')).toBe(0)
  })

  it('matches whatever the case', () => {
    expect(score('src/Main.rs', 'main')).toBeGreaterThan(0)
  })

  it('lets everything through on an empty query', () => {
    expect(score('anything', '')).toBe(1)
  })
})

describe('what the field lists', () => {
  const files = ['src/domain.rs', 'src/main.rs', 'main/util.rs']

  it('puts the best match first', () => {
    expect(ranked(files, 'main', (one) => one)[0]).toBe('src/main.rs')
  })

  it('leaves out what does not match', () => {
    expect(ranked(files, 'domain', (one) => one)).toEqual(['src/domain.rs'])
  })

  /* A list that reshuffles between keystrokes is a list you cannot click. */
  it('keeps the order it was given when scores tie', () => {
    expect(ranked(['b.rs', 'a.rs'], '.rs', (one) => one)).toEqual(['b.rs', 'a.rs'])
  })

  it('stops at the ceiling it was given', () => {
    expect(ranked(files, '', (one) => one, 2)).toHaveLength(2)
  })
})
