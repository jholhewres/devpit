import { describe, expect, it } from 'vitest'

import type { Secret } from '../gen/bindings'
import { rotationChoice, rotationDay, rotationMoment, rotationSaid } from './secretRotation'

const secret = (over: Partial<Secret> = {}): Secret => ({ name: 'HF_TOKEN', setAt: 0, rotateDays: null, rotateAt: null, reminderCard: null, ...over })

describe("a secret's rotation", () => {
  it('goes off at nine in the morning of the day picked', () => {
    const at = rotationMoment('2027-01-09')
    expect(at).not.toBeNull()
    const moment = new Date(at! * 1000)
    expect([moment.getFullYear(), moment.getMonth(), moment.getDate(), moment.getHours()]).toEqual([2027, 0, 9, 9])
    expect(rotationDay(at!)).toBe('2027-01-09')
    expect(rotationMoment('')).toBeNull()
  })

  it('says when it is due, and that it is overdue', () => {
    expect(rotationSaid(secret(), 100)).toBeNull()
    expect(rotationSaid(secret({ rotateAt: 200 }), 100)).toMatch(/^rotate by /)
    expect(rotationSaid(secret({ rotateAt: 50 }), 100)).toMatch(/^rotation due since /)
  })

  it('is an interval, a date alone, or none', () => {
    expect(rotationChoice(secret())).toBe('')
    expect(rotationChoice(secret({ rotateDays: 90, rotateAt: 1 }))).toBe('90')
    expect(rotationChoice(secret({ rotateAt: 1 }))).toBe('date')
  })
})
