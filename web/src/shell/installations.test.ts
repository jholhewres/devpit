import { describe, expect, it } from 'vitest'

import { named } from './useInstallations'

describe('what an installation is called', () => {
  it('is its profiles when any run against it', () => {
    expect(named({ directory: '/home/me/.claude-glm', profiles: ['glm', 'glm-fast'], default: false })).toBe(
      'glm · glm-fast',
    )
  })

  it('is its folder, without the CLI prefix, when none do', () => {
    expect(named({ directory: '/home/me/.claude-work', profiles: [], default: false })).toBe('work')
    expect(named({ directory: '/home/me/.claude', profiles: [], default: true })).toBe('claude')
    expect(named({ directory: '/opt/cli-config/', profiles: [], default: false })).toBe('cli-config')
  })
})
