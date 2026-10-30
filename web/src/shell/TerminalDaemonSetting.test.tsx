import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const calls = vi.hoisted(() => ({ set: [] as boolean[], on: false }))

vi.mock('./live', () => ({
  ask: <T,>(run: () => Promise<T>) => run(),
  commands: {
    terminalDaemonRead: () => Promise.resolve({ data: calls.on }),
    terminalDaemonSet: (on: boolean) => {
      calls.set.push(on)
      calls.on = on
      return Promise.resolve({ data: on })
    },
  },
}))

import { TerminalDaemonSetting } from './TerminalDaemonSetting'

describe('TerminalDaemonSetting', () => {
  beforeEach(() => {
    calls.set = []
    calls.on = false
  })

  it('is off until turned on, and turning it on asks the desktop', async () => {
    render(<TerminalDaemonSetting />)
    const toggle = await screen.findByRole('switch')
    expect(toggle.getAttribute('aria-checked')).toBe('false')
    fireEvent.click(toggle)
    await waitFor(() => expect(toggle.getAttribute('aria-checked')).toBe('true'))
    expect(calls.set).toEqual([true])
  })
})
