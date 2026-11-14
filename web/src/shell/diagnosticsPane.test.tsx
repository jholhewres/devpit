import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Diagnostics } from '../gen/bindings'
import { DiagnosticsPane } from './DiagnosticsPane'

const GB = 1024 * 1024 * 1024
const seen: Diagnostics = {
  processes: [
    { pid: 4100, name: 'devpit-desktop', cpuTenths: 105, memoryBytes: 1.5 * GB },
    { pid: 4200, name: 'devpit-ptyd', cpuTenths: 20, memoryBytes: 0.5 * GB },
  ],
  checks: [
    { name: 'devpit is open', ok: true, detail: 'it answers', fix: null },
    { name: 'the disk has room', ok: false, detail: '2.0 GB free of 500.0 GB', fix: 'free space on the disk that holds /home/me/.devpit' },
  ],
  log: ['background · stall_watch: the page has not run for 14s'],
  logPath: '/home/me/.devpit/logs/errors.jsonl',
}
const read = vi.fn(async () => seen)
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: { diagnosticsRead: () => read() },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

describe('Settings → Diagnostics', () => {
  it("lists devpit's processes with their CPU and memory, and the totals", async () => {
    render(<DiagnosticsPane />)
    expect(await screen.findByText('2 processes, 12.5% of a core, 2.0 GB of memory.')).toBeTruthy()
    const row = screen.getByText('devpit-desktop').closest('tr')
    expect(row?.textContent).toBe('devpit-desktop410010.5%1.5 GB')
    expect(screen.getByText('devpit-ptyd')).toBeTruthy()
  })

  it('shows each check, with the fix beside one that failed', async () => {
    render(<DiagnosticsPane />)
    expect(await screen.findByText('✓ devpit is open')).toBeTruthy()
    expect(screen.getByText('it answers')).toBeTruthy()
    expect(screen.getByText('2.0 GB free of 500.0 GB — fix: free space on the disk that holds /home/me/.devpit')).toBeTruthy()
  })

  it("shows where the error log is and its newest lines", async () => {
    render(<DiagnosticsPane />)
    expect(await screen.findByText('/home/me/.devpit/logs/errors.jsonl')).toBeTruthy()
    expect(screen.getByText('background · stall_watch: the page has not run for 14s')).toBeTruthy()
  })

  it('looks again only when asked', async () => {
    render(<DiagnosticsPane />)
    await screen.findByText('devpit-desktop')
    expect(read).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByRole('button', { name: 'Look again' }))
    await waitFor(() => expect(read).toHaveBeenCalledTimes(2))
  })
})
