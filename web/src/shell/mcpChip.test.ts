import { describe, expect, it } from 'vitest'

import type { McpServerHealth } from '../gen/bindings'
import { summed } from './McpChip'

const server = (state: McpServerHealth['state']): McpServerHealth => ({ name: state, target: 'x', state, detail: null })

describe('summed', () => {
  it('counts only the connected, and anything else asks to be looked at', () => {
    expect(summed([server('connected'), server('connected')])).toEqual({ up: 2, all: 2, bad: false })
    expect(summed([server('connected'), server('needs_auth'), server('failed')])).toEqual({ up: 1, all: 3, bad: true })
  })
})
