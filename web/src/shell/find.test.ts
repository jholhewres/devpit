import { describe, expect, it } from 'vitest'

import { filter, looksLikeDotfile, type FindFlags } from './find'

const off: FindFlags = { case: false, word: false, regex: false }

const paths = [
  'web/src/App.tsx',
  'app.tsx',
  'src/useTree.ts',
  'src/tree.ts',
  'web/src/shell/shell.css',
]

const pathsOf = (query: string, flags: FindFlags): string[] =>
  filter(paths, query, flags).hits.map((hit) => hit.path)

describe('matching a name across the whole project', () => {
  it('ignores case with match-case off', () => {
    expect(pathsOf('app', off)).toContain('web/src/App.tsx')
  })

  it('with match-case on, a different case is not a match', () => {
    const hits = pathsOf('App', { ...off, case: true })
    expect(hits).toContain('web/src/App.tsx')
    expect(hits).not.toContain('app.tsx')
  })

  it('with whole-word on, the query has to stand on its own', () => {
    const hits = pathsOf('tree', { ...off, word: true })
    expect(hits).toContain('src/tree.ts')
    expect(hits).not.toContain('src/useTree.ts')
  })

  it('with regex on, the pattern runs against the whole path', () => {
    expect(pathsOf('^web/.*\\.css$', { ...off, regex: true })).toEqual(['web/src/shell/shell.css'])
  })

  it('reports a broken pattern instead of throwing', () => {
    expect(() => filter(paths, '(', { ...off, regex: true })).not.toThrow()
    const result = filter(paths, '(', { ...off, regex: true })
    expect(result.hits).toHaveLength(0)
    expect(result.reason).not.toBeNull()
  })

  it('with regex and whole-word both on, the boundary wraps the whole pattern', () => {
    /* An ungrouped `\bfoo|bar\b` would let "foo" match inside "foobar.ts",
       because the trailing boundary binds only to the "bar" branch. */
    const hits = filter(['foo.ts', 'bar.ts', 'foobar.ts'], 'foo|bar', { ...off, word: true, regex: true }).hits.map(
      (hit) => hit.path,
    )
    expect(hits).toEqual(expect.arrayContaining(['foo.ts', 'bar.ts']))
    expect(hits).not.toContain('foobar.ts')
  })

  it('keeps nothing for an empty query', () => {
    expect(filter(paths, '   ', off).hits).toHaveLength(0)
  })
})

describe('naming a query that looks like a dotfile', () => {
  it('says yes for a bare dotfile name', () => {
    expect(looksLikeDotfile('.gitignore')).toBe(true)
  })

  it('says yes for a dotfile inside a path', () => {
    expect(looksLikeDotfile('src/.env')).toBe(true)
  })

  it('says no for an ordinary name', () => {
    expect(looksLikeDotfile('tree.ts')).toBe(false)
  })
})
