import { describe, expect, it } from 'vitest'

import { repeatSaid, stateSaid } from './schedules'

const routine = (days: number[]) => ({ kind: 'morning' as const, minute: 540, days, zone: 'UTC', offsetMinutes: 0 })

describe('what a Schedules row says', () => {
  it('says whether it repeats', () => {
    expect(repeatSaid(null)).toBe('Once')
    expect(repeatSaid(routine([]))).toBe('Every day')
    expect(repeatSaid(routine([5, 1, 2, 3, 4]))).toBe('Weekdays')
    expect(repeatSaid(routine([6, 0]))).toBe('Weekends')
    expect(repeatSaid(routine([1, 3]))).toBe('Mon, Wed')
  })

  it('says its state in words', () => {
    expect(stateSaid('due')).toBe('Went off')
    expect(stateSaid('paused')).toBe('Paused')
  })
})
