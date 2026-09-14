import { describe, expect, it } from 'vitest'

import { afterSession, installationOf } from './paneSessions'

const said = (sessionId: string, transcript: string): string => JSON.stringify({ sessionId, transcript })

describe('the session a pane is in', () => {
  it('takes the installation from the transcript folder', () => {
    expect(installationOf('/home/me/.claude-glm/projects/-home-me-work/abc.jsonl')).toBe('/home/me/.claude-glm')
    // A project folder has its slashes replaced, so only the installation's own `/projects/` counts.
    expect(installationOf('/home/me/projects/.claude/projects/-home-me-projects-x/abc.jsonl')).toBe(
      '/home/me/projects/.claude',
    )
    expect(installationOf('/nowhere.jsonl')).toBeNull()
  })

  it('keeps the latest session per pane and ignores what it cannot use', () => {
    const one = afterSession({}, 'leaf_1', said('abc', '/home/me/.claude/projects/-w/abc.jsonl'))
    expect(one.leaf_1).toEqual({ sessionId: 'abc', installation: '/home/me/.claude' })
    expect(afterSession(one, 'leaf_1', said('abc', '/home/me/.claude/projects/-w/abc.jsonl'))).toBe(one)
    for (const bad of [null, 'not json', said('', '/a/projects/b.jsonl'), said('abc', '/no-folder.jsonl')]) {
      expect(afterSession(one, 'leaf_1', bad)).toBe(one)
    }
    // `/clear` starts another session in the same pane.
    expect(afterSession(one, 'leaf_1', said('def', '/home/me/.claude/projects/-w/def.jsonl')).leaf_1?.sessionId).toBe('def')
  })
})
