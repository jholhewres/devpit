import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import type { Terminal } from '@xterm/xterm'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { TerminalMenu } from './TerminalMenu'

const opened = vi.fn((projectId: string, paneId: string) => ({
  session: `ptyd-view_${paneId}`,
  attach: `tmux -S /h/tmux.sock attach -t ptyd-view_${paneId} # ${projectId}`,
}))
const copied = vi.fn((_line: string) => Promise.resolve())

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: { paneOpenInTmux: (projectId: string, paneId: string) => opened(projectId, paneId) },
}))
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({ writeText: (line: string) => copied(line) }))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

describe('Open in tmux', () => {
  const terminal = { hasSelection: () => false, focus: vi.fn(), selectAll: vi.fn() } as unknown as Terminal

  it('opens the pane in tmux and copies and shows the line that attaches to it', async () => {
    const told = vi.fn()
    render(<TerminalMenu at={{ x: 0, y: 0 }} terminal={terminal} projectId="p" paneId="leaf_1" onClose={vi.fn()} onFailed={told} />)
    fireEvent.click(screen.getByRole('menuitem', { name: 'Open in tmux' }))
    await waitFor(() => expect(told).toHaveBeenCalled())
    expect(opened).toHaveBeenCalledWith('p', 'leaf_1')
    const line = 'tmux -S /h/tmux.sock attach -t ptyd-view_leaf_1 # p'
    expect(copied).toHaveBeenCalledWith(line)
    expect(told).toHaveBeenCalledWith(`Copied: ${line}`)
  })

  it('is not offered without a pane to open', () => {
    render(<TerminalMenu at={{ x: 0, y: 0 }} terminal={terminal} projectId="p" onClose={vi.fn()} onFailed={vi.fn()} />)
    expect(screen.queryByRole('menuitem', { name: 'Open in tmux' })).toBeNull()
  })
})
