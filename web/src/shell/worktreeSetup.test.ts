import { describe, expect, it } from 'vitest'

import { envOf } from './WorktreeSetupFields'

describe('envOf', () => {
  it('reads NAME=value per line, keeping an = inside the value', () => {
    expect(envOf('A=1\n\nnot a variable\nURL=https://x?a=b\n=nameless')).toEqual([
      { name: 'A', value: '1' },
      { name: 'URL', value: 'https://x?a=b' },
    ])
  })
})
