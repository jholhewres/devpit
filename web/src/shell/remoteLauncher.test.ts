import { describe, expect, it } from 'vitest'

import { launcherLink, machineName } from './remoteLauncher'

describe('the link that adds a machine to devpit.app/remote', () => {
  it('carries only its address and name, in the fragment', () => {
    const link = launcherLink('https://box.tail1234.ts.net/')
    expect(link).toBe('https://devpit.app/remote#add=https%3A%2F%2Fbox.tail1234.ts.net&name=box')
    expect(new URL(link!).search).toBe('')
  })

  it('needs the tailnet HTTPS address', () => {
    expect(launcherLink('http://100.64.0.7:7777')).toBeNull()
    expect(launcherLink('not an address')).toBeNull()
  })

  it('names the machine by its first label', () => {
    expect(machineName('https://box.tail1234.ts.net')).toBe('box')
  })
})
