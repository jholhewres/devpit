import { describe, expect, it } from 'vitest'

import { deviceName } from './deviceName'

const LINUX_FIREFOX = 'Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0'
const IPADOS = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15'
const CHROMEBOOK = 'Mozilla/5.0 (X11; CrOS x86_64 14541.0.0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36'
const ANDROID = 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Mobile Safari/537.36'

describe('the name a device offers when it pairs', () => {
  it('names the system, so the row in Settings says which device it is', () => {
    expect(deviceName('Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15')).toBe('iPhone')
    expect(deviceName(ANDROID)).toBe('Android')
    expect(deviceName(LINUX_FIREFOX)).toBe('Linux')
    expect(deviceName(CHROMEBOOK)).toBe('Chromebook')
    expect(deviceName('Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0')).toBe('Windows')
  })

  it('tells an iPad asking for the desktop site from a Mac by its touch screen', () => {
    expect(deviceName(IPADOS, 5)).toBe('iPad')
    expect(deviceName(IPADOS, 0)).toBe('Mac')
  })

  it('falls back to the browser, never to a name that fits every device', () => {
    expect(deviceName('Mozilla/5.0 (Unknown) Firefox/128.0')).toBe('Firefox browser')
    expect(deviceName('')).toBe('A browser')
  })
})
