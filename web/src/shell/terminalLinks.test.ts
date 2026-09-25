import { describe, expect, it } from 'vitest'

import { openable, pathsIn, urlsIn } from './terminalLinks'

describe('paths in a terminal line', () => {
  it('finds the file an agent read, a path with a line, and a home path', () => {
    const line = '● Read(/home/jhol/Workspace/app/.playwright-mcp/home-mobile-full.jpeg) and src/a.ts:12 and ~/logs/run.log'
    const found = pathsIn(line)
    expect(found.map((one) => one.path)).toEqual([
      '/home/jhol/Workspace/app/.playwright-mcp/home-mobile-full.jpeg',
      'src/a.ts',
      '~/logs/run.log',
    ])
    expect(found[1]!.line).toBe(12)
    expect(line.slice(found[0]!.start, found[0]!.end)).toBe(found[0]!.path)
  })

  it('leaves words with a dot and URLs alone', () => {
    expect(pathsIn('version 1.2.3 of e.g. this')).toEqual([])
    expect(pathsIn('see https://example.com/a/b.js')).toEqual([])
  })

  it('takes a path whole, not up to the first dot in it', () => {
    expect(pathsIn('~/.config/devpit/x')).toEqual([])
    expect(pathsIn('run node_modules/.bin/vite now')).toEqual([])
    expect(pathsIn('foo/bar.baz/qux.ts').map((one) => one.path)).toEqual(['foo/bar.baz/qux.ts'])
  })

  it('leaves the inside of any URL alone', () => {
    expect(pathsIn('at http://localhost:3000/x.js')).toEqual([])
    expect(pathsIn('https://ex.com/a%20b/c.js')).toEqual([])
    expect(pathsIn('www.example.com/a.js')).toEqual([])
    expect(pathsIn('see https://x.io and src/a.ts').map((one) => one.path)).toEqual(['src/a.ts'])
  })

  it('opens a relative path where the shell is, as one tab however it was written', () => {
    expect(openable('src/a.ts', null, '/w/app', '/w/app/web')).toBe('web/src/a.ts')
    expect(openable('./src/a.ts', null, '/w/app', '/w/app')).toBe('src/a.ts')
    expect(openable('../x.ts', null, '/w/app', '/w/app/web')).toBe('x.ts')
    expect(openable('src/a.ts', null, '/w/app', '/h/.devpit/worktrees/c1')).toBe('/h/.devpit/worktrees/c1/src/a.ts')
  })

  it('opens a project file by its project path, and the rest where they are', () => {
    expect(openable('/w/app/src/a.ts', '/home/me', '/w/app', null)).toBe('src/a.ts')
    expect(openable('~/x/y.log', '/home/me', '/w/app', null)).toBe('/home/me/x/y.log')
    expect(openable('src/a.ts', null, '/w/app', '/w/app')).toBe('src/a.ts')
    expect(openable('/tmp/shot.png', '/home/me', '/w/app', null)).toBe('/tmp/shot.png')
  })
})

describe('web addresses in a terminal line', () => {
  it('are found where they are, and a path inside one is not a file', () => {
    const line = 'open https://claude.ai/code/session_01/x.md now'
    expect(urlsIn(line)).toEqual([{ start: 5, end: 43, url: 'https://claude.ai/code/session_01/x.md' }])
    expect(pathsIn(line)).toEqual([])
  })
})
