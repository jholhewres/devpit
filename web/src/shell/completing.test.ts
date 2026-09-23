import { describe, expect, it } from 'vitest'

import { common, finished, wordAt } from './completing'

describe('Tab in the terminal’s editor', () => {
  it('finds the word the cursor ends', () => {
    expect(wordAt('cd src/ma', 9)).toEqual({ start: 3, word: 'src/ma' })
    expect(wordAt('ls ', 3)).toEqual({ start: 3, word: '' })
  })

  it('goes as far as every choice agrees', () => {
    expect(common(['src/main.rs', 'src/mod.rs'])).toBe('src/m')
    expect(common(['only/'])).toBe('only/')
    expect(common([])).toBe('')
  })

  it('puts the choice in place of the word', () => {
    expect(finished('cat sr | wc', 4, 6, 'src/')).toEqual({ text: 'cat src/ | wc', caret: 8 })
  })
})
