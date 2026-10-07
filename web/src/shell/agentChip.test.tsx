import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { AgentHealth } from '../gen/bindings'

const restarted = vi.fn()
const health = (over: Partial<AgentHealth>): AgentHealth => ({
  answering: true,
  latencyMs: 12.4,
  checkedAt: 1,
  failures: 0,
  restarts: 0,
  lastRestartAt: null,
  detail: null,
  ...over,
})
let now = health({})

vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    agentHealth: async () => now,
    agentRestart: async () => (restarted(), (now = health({ restarts: 1 }))),
  },
}))
vi.mock('./window', () => ({ onAgentHealth: () => () => {} }))

const { AgentChip, agentWord } = await import('./AgentChip')

afterEach(cleanup)

describe("devpit's own MCP in the footer", () => {
  it('says how long it took to answer, or that it did not', () => {
    expect(agentWord(health({ latencyMs: 12.4 }))).toBe('12 ms')
    expect(agentWord(health({ answering: false, latencyMs: null, failures: 2 }))).toBe('no answer')
  })

  it('is restarted from its chip, and says so', async () => {
    now = health({ answering: false, latencyMs: null, failures: 2, detail: 'timed out' })
    render(<AgentChip />)
    fireEvent.click(await screen.findByTitle("devpit's own MCP"))
    expect(screen.getByText(/Not answering: 2 checks in a row/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Restart MCP' }))
    expect(await screen.findByText(/Restarted 1 time\./)).toBeTruthy()
    expect(restarted).toHaveBeenCalledOnce()
  })
})
