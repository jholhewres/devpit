import { describe, expect, it } from 'vitest'

import { launchShown } from './launchLine'

describe('launchShown', () => {
  it('leaves out the variables in front of the program', () => {
    expect(launchShown("CLAUDE_CONFIG_DIR='/home/me/.claude-work' claude --permission-mode plan")).toBe('claude --permission-mode plan')
    expect(launchShown('A=1 B="two words" TOKEN=x claude')).toBe('claude')
  })

  it('keeps a line with nothing in front as it is', () => {
    expect(launchShown('codex --full-auto')).toBe('codex --full-auto')
  })
})
