import { describe, expect, it } from 'vitest'

import { formatHere, refusal, systemLanguage } from './recorder'

describe('recording a voice message', () => {
  it('records nothing where the window has no recorder', () => {
    expect(formatHere()).toBeNull()
  })

  it('hears in the system language by its first part', () => {
    const was = navigator.language
    Object.defineProperty(navigator, 'language', { value: 'pt-BR', configurable: true })
    expect(systemLanguage()).toBe('pt')
    Object.defineProperty(navigator, 'language', { value: was, configurable: true })
  })

  it('says how to unblock a refused microphone', () => {
    expect(refusal(new DOMException('no', 'NotAllowedError'))).toMatch(/privacy settings/)
    expect(refusal(new DOMException('no', 'NotFoundError'))).toBe('No microphone was found.')
  })
})
