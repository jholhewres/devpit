import { describe, expect, it } from 'vitest'

import { cueOf } from './chime'

describe('the island’s sounds', () => {
  it('sound once when a session stops on you, and not again while it waits', () => {
    expect(cueOf('working', 'waiting')).toBe('waiting')
    expect(cueOf('waiting', 'waiting')).toBeNull()
  })

  /* A sound per tool call is a sound somebody switches off for good. */
  it('stay quiet while work goes on', () => {
    expect(cueOf('open', 'working')).toBeNull()
    expect(cueOf(undefined, 'working')).toBeNull()
    expect(cueOf('working', 'working')).toBeNull()
  })

  it('mark a turn that finished only when it was seen working', () => {
    expect(cueOf('working', 'done')).toBe('done')
    expect(cueOf(undefined, 'done')).toBeNull()
  })

  it('mark a failure', () => {
    expect(cueOf('working', 'failed')).toBe('failed')
  })
})
