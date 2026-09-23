import { describe, expect, it } from 'vitest'

import type { ClaudePluginInstallation } from '../gen/bindings'
import { offerFor } from './claudePlugin'

const one = (state: ClaudePluginInstallation['state']): ClaudePluginInstallation => ({ directory: `/h/${state}`, profiles: [], state })

describe('offerFor', () => {
  it('offers nothing once every installation has the current plugin', () => {
    expect(offerFor([one('current'), one('current')])).toBeNull()
    expect(offerFor([])).toBeNull()
  })

  it('offers an install while any installation lacks it', () => {
    expect(offerFor([one('current'), one('missing'), one('outdated')])).toBe('install')
  })

  it('offers an update when every installation has it but one is behind', () => {
    expect(offerFor([one('current'), one('outdated')])).toBe('update')
  })
})
