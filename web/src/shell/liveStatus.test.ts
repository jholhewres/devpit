import { describe, expect, it } from 'vitest'

import type { LiveSession } from '../gen/bindings'
import { inOrder } from './liveStatus'

const session = (name: string, status: string): LiveSession => ({ name, status }) as unknown as LiveSession

describe('the sessions beside an orchestrator', () => {
  it('put the busy ones first, then go by name', () => {
    const listed = inOrder([session('b-idle', 'idle'), session('z-busy', 'busy'), session('a-idle', 'idle'), session('c-busy', 'busy')])
    expect(listed.map((one) => one.name)).toEqual(['c-busy', 'z-busy', 'a-idle', 'b-idle'])
  })
})
