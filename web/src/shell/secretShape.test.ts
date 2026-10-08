import { describe, expect, it } from 'vitest'

import { looksSecret } from './secretShape'

describe('looksSecret', () => {
  it('sees a key by its shape, bare or assigned', () => {
    expect(looksSecret('use JEV_API_KEY=sk-abcdefghijklmnopqrstuvwx please')).toBe(true)
    expect(looksSecret('AKIAIOSFODNN7EXAMPLE')).toBe(true)
    expect(looksSecret('-----BEGIN OPENSSH PRIVATE KEY-----\nabc')).toBe(true)
  })

  it('leaves ordinary words alone', () => {
    expect(looksSecret('install sk-learn and run the task-runner')).toBe(false)
    expect(looksSecret('envie')).toBe(false)
  })
})
