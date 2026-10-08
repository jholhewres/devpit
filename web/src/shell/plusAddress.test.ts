import { describe, expect, it } from 'vitest'

import { plusLink } from './plusAddress'

describe('plusLink', () => {
  it('says only where it was clicked', () => {
    expect(plusLink('app-remote')).toBe('https://devpit.app/pro?from=app-remote')
  })
})
