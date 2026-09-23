import { describe, expect, it } from 'vitest'

import { inOrder } from './inOrder'

describe('inOrder', () => {
  it('sends a write only once the one before it has answered', async () => {
    const landed: string[] = []
    let release: () => void = () => {}
    const first = inOrder('k', () => new Promise<void>((done) => (release = () => (landed.push('first'), done()))))
    const second = inOrder('k', async () => void landed.push('second'))
    await Promise.resolve()
    expect(landed).toEqual([])
    release()
    await Promise.all([first, second])
    expect(landed).toEqual(['first', 'second'])
  })

  it('keeps going after a write that failed', async () => {
    await inOrder('j', () => Promise.reject(new Error('no'))).catch(() => undefined)
    expect(await inOrder('j', async () => 'next')).toBe('next')
  })
})
