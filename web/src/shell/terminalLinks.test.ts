import { describe, expect, it } from 'vitest'

import { openable, pathsIn } from './terminalLinks'

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

  it('opens a project file by its project path, and the rest where they are', () => {
    expect(openable('/w/app/src/a.ts', '/home/me', '/w/app', null)).toBe('src/a.ts')
    expect(openable('~/x/y.log', '/home/me', '/w/app', null)).toBe('/home/me/x/y.log')
    expect(openable('src/a.ts', null, '/w/app', '/w/app')).toBe('src/a.ts')
    expect(openable('/tmp/shot.png', '/home/me', '/w/app', null)).toBe('/tmp/shot.png')
  })
})
