import { describe, expect, it } from 'vitest'

import { who } from './account'
import type { Account } from '../gen/bindings'

function account(fields: Partial<Account> = {}): Account {
  return {
    id: '01ABC',
    email: 'jhol.code@gmail.com',
    name: null,
    createdAt: '2026-09-10T00:00:00Z',
    ...fields,
  }
}

describe('who the window says you are', () => {
  it('says nothing it does not know when there is no account', () => {
    /* A fabricated name is worse than an honest blank. */
    expect(who(null)).toEqual({ name: 'Signed in', email: null, initials: '·' })
  })

  it('takes the name from the address when the server has no name', () => {
    expect(who(account()).name).toBe('jhol.code')
  })

  it('prefers the name the person set', () => {
    // They chose one of the two; the address is the fallback, not the answer.
    expect(who(account({ name: 'Jhol Hewres' })).name).toBe('Jhol Hewres')
    expect(who(account({ name: 'Jhol Hewres' })).initials).toBe('JH')
  })

  it('ignores a name that is only whitespace', () => {
    expect(who(account({ name: '   ' })).name).toBe('jhol.code')
  })

  it('builds initials from the parts of the name', () => {
    expect(who(account()).initials).toBe('JC')
  })

  it('falls back to one letter when there is only one part', () => {
    expect(who(account({ email: 'jhol@gmail.com' })).initials).toBe('J')
  })

  it('keeps the whole address as the email', () => {
    expect(who(account({ email: 'a.b@c.com' })).email).toBe('a.b@c.com')
  })
})
