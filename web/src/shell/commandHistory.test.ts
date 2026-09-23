import { describe, expect, it } from 'vitest'

import { searched, suggestion, withLine } from './commandHistory'

describe('the terminal’s history', () => {
  it('keeps the newest first, once each, and nothing blank', () => {
    expect(withLine(['ls', 'git status'], 'git status')).toEqual(['git status', 'ls'])
    expect(withLine(['ls'], '   ')).toEqual(['ls'])
  })

  it('suggests the rest of the newest line that starts with what was typed', () => {
    const history = ['git status', 'git stash pop', 'ls']
    expect(suggestion(history, 'git st')).toBe('atus')
    expect(suggestion(history, 'git status')).toBe(null)
    expect(suggestion(history, '')).toBe(null)
    expect(suggestion(history, 'nope')).toBe(null)
  })

  it('searches by every word, in any order', () => {
    expect(searched(['npm run build', 'npm test', 'cargo build'], 'build npm')).toEqual(['npm run build'])
  })
})
