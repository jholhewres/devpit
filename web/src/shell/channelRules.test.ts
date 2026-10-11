import { describe, expect, it } from 'vitest'

import type { ChannelRules } from '../gen/bindings'
import { clock, escalated, flipped, minutes } from './channelRules'

const rules: ChannelRules = { routes: [{ event: 'session_waiting', channels: ['telegram'], escalateAfter: 10 }], quiet: null, groupSeconds: 60 }

describe('channel rules', () => {
  it('switches one channel for one event and leaves the rest', () => {
    const off = flipped(rules, 'session_waiting', 'telegram', false)
    expect(off.routes.find((one) => one.event === 'session_waiting')?.channels).toEqual([])
    const on = flipped(off, 'reminder', 'telegram', true)
    expect(on.routes.find((one) => one.event === 'reminder')?.channels).toEqual(['telegram'])
    expect(on.routes).toHaveLength(7)
  })

  it('sets one event’s minutes before moving on and keeps its channels', () => {
    const now = escalated(rules, 'session_waiting', 30)
    expect(now.routes.find((one) => one.event === 'session_waiting')).toEqual({ event: 'session_waiting', channels: ['telegram'], escalateAfter: 30 })
    const atOnce = escalated(now, 'session_waiting', null)
    expect(atOnce.routes.find((one) => one.event === 'session_waiting')?.escalateAfter).toBeNull()
    expect(flipped(now, 'session_waiting', 'devpit_app', true).routes[0].escalateAfter).toBe(30)
  })

  it('reads and writes a clock', () => {
    expect(minutes('22:30')).toBe(1350)
    expect(clock(450)).toBe('07:30')
  })
})
