import { describe, expect, it } from 'vitest'

import { flagOn } from './settingsFlags'

describe('a switch nobody has touched', () => {
  it('is on for updates and the stop prompt, off for what is opt-in', () => {
    expect(flagOn(null, 'automaticUpdates')).toBe(true)
    expect(flagOn(null, 'confirmStop')).toBe(true)
    expect(flagOn(null, 'focusMode')).toBe(false)
    expect(flagOn({ errorReports: null }, 'errorReports')).toBe(false)
  })

  it('is what the person chose once they chose', () => {
    expect(flagOn({ errorReports: true }, 'errorReports')).toBe(true)
    expect(flagOn({ automaticUpdates: false }, 'automaticUpdates')).toBe(false)
  })
})
