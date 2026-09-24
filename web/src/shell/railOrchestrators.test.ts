import { describe, expect, it } from 'vitest'

import type { Profile, Thread } from '../gen/bindings'
import { lastSpoken, orchestrable } from './RailOrchestrators'

const profile = (driver: string, path: string | null): Profile => ({ driver, path }) as unknown as Profile
const thread = (id: string, lastAt: number | null): Thread => ({ id, lastAt }) as unknown as Thread

describe('orchestrators in the rail', () => {
  it('are offered only for Claude Code accounts that are installed', () => {
    expect(orchestrable(profile('claude', '/usr/bin/claude'))).toBe(true)
    expect(orchestrable(profile('codex', '/usr/bin/codex'))).toBe(false)
    expect(orchestrable(profile('claude', null))).toBe(false)
  })

  it('pick back up the conversation last spoken in', () => {
    expect(lastSpoken([thread('old', 10), thread('new', 30), thread('never', null)])?.id).toBe('new')
    expect(lastSpoken([])).toBeUndefined()
  })
})
