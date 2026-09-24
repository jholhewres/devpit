import { describe, expect, it } from 'vitest'

import type { Profile, Thread } from '../gen/bindings'
import { claudeAccounts, needsReading } from './OrchestratorDialog'
import { lastSpoken } from './RailOrchestrators'

const profile = (driver: string, path: string | null, reach = 'runnable'): Profile => ({ driver, path, reach }) as unknown as Profile
const thread = (id: string, lastAt: number | null): Thread => ({ id, lastAt }) as unknown as Thread

describe('making an orchestrator', () => {
  it('offers every Claude Code account, a shell function devpit has not read included', () => {
    const offered = claudeAccounts([
      profile('claude', '/usr/bin/claude'),
      profile('claude', null, 'shell_only'),
      profile('claude', null, 'missing'),
      profile('codex', '/usr/bin/codex'),
    ])
    expect(offered).toHaveLength(2)
    expect(offered.filter(needsReading)).toHaveLength(1)
  })

  it('picks back up the conversation last spoken in', () => {
    expect(lastSpoken([thread('old', 10), thread('new', 30), thread('never', null)])?.id).toBe('new')
    expect(lastSpoken([])).toBeUndefined()
  })
})
