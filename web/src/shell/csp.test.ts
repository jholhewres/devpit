import { afterEach, describe, expect, it, vi } from 'vitest'

import { watchCsp } from './csp'

afterEach(() => vi.restoreAllMocks())

describe('a blocked request', () => {
  it('names the directive that refused it and what it refused', () => {
    const said = vi.spyOn(console, 'error').mockImplementation(() => {})
    const stop = watchCsp(document)

    const blocked = Object.assign(new Event('securitypolicyviolation'), {
      violatedDirective: 'connect-src',
      blockedURI: 'https://example.invalid/track',
    })
    document.dispatchEvent(blocked)

    expect(said).toHaveBeenCalledWith('[csp] connect-src blocked https://example.invalid/track')
    stop()
  })

  /* An inline script has no URI, and "blocked " with nothing after it reads
     like the log itself is broken. */
  it('says so when there is no url to name', () => {
    const said = vi.spyOn(console, 'error').mockImplementation(() => {})
    const stop = watchCsp(document)

    document.dispatchEvent(
      Object.assign(new Event('securitypolicyviolation'), {
        violatedDirective: 'script-src',
        blockedURI: '',
      }),
    )

    expect(said).toHaveBeenCalledWith('[csp] script-src blocked something inline')
    stop()
  })

  /* Stopping means stopping: a listener left behind would report a violation
     twice once a second screen starts watching. */
  it('stops reporting once it is let go', () => {
    const said = vi.spyOn(console, 'error').mockImplementation(() => {})
    watchCsp(document)()

    document.dispatchEvent(
      Object.assign(new Event('securitypolicyviolation'), {
        violatedDirective: 'img-src',
        blockedURI: 'https://example.invalid/pixel.png',
      }),
    )

    expect(said).not.toHaveBeenCalled()
  })
})
