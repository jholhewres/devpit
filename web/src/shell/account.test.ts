import { describe, expect, it } from 'vitest'

import { who } from './account'

describe('who the window says you are', () => {
  it('says nothing it does not know when there is no account', () => {
    /* A fabricated name is worse than an honest blank. */
    expect(who(null)).toEqual({ name: 'Signed in', email: null, initials: '·' })
  })

  it('takes the name from the address', () => {
    expect(who('jhol.code@gmail.com').name).toBe('jhol.code')
  })

  it('builds initials from the parts of the name', () => {
    expect(who('jhol.code@gmail.com').initials).toBe('JC')
  })

  it('falls back to one letter when there is only one part', () => {
    expect(who('jhol@gmail.com').initials).toBe('J')
  })

  it('keeps the whole address as the email', () => {
    expect(who('a.b@c.com').email).toBe('a.b@c.com')
  })
})
