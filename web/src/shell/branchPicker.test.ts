import { describe, expect, it } from 'vitest'

import type { Branch } from '../gen/bindings'
import { matching } from './BranchPicker'

const branch = (name: string, subject: string): Branch => ({ name, subject, current: false })

describe('matching', () => {
  const all = [branch('main', 'chore: v0.1.24'), branch('feat/LED-1-media', 'media upload'), branch('fix/login', 'LED-2 login')]

  it('keeps every branch when nothing is typed', () => {
    expect(matching(all, '  ')).toHaveLength(3)
  })

  it('needs every word, in the name or the last subject', () => {
    expect(matching(all, 'led').map((one) => one.name)).toEqual(['feat/LED-1-media', 'fix/login'])
    expect(matching(all, 'led login').map((one) => one.name)).toEqual(['fix/login'])
  })
})
