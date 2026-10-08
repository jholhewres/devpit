import { describe, expect, it } from 'vitest'

import type { ChannelRules } from '../gen/bindings'
import { clock, flipped, minutes } from './channelRules'

const rules: ChannelRules = { routes: [{ event: 'session_waiting', channels: ['telegram'] }], quiet: null, groupSeconds: 60 }

describe('channel rules', () => {
  it('switches one channel for one event and leaves the rest', () => {
    const off = flipped(rules, 'session_waiting', 'telegram', false)
    expect(off.routes.find((one) => one.event === 'session_waiting')?.channels).toEqual([])
    const on = flipped(off, 'reminder', 'telegram', true)
    expect(on.routes.find((one) => one.event === 'reminder')?.channels).toEqual(['telegram'])
    expect(on.routes).toHaveLength(7)
  })

  it('reads and writes a clock', () => {
    expect(minutes('22:30')).toBe(1350)
    expect(clock(450)).toBe('07:30')
  })
})
