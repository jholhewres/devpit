import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { LiveSession } from '../gen/bindings'
import { PANE_FREED } from './Leaf'
import { SessionTerminal } from './SessionTerminal'

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: { terminalClose: () => null, sessionChanges: () => null },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ show: vi.fn(), openCard: vi.fn() }) }))
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({ writeText: vi.fn(() => Promise.resolve()) }))
// The terminal itself is not what is under test, and xterm wants a canvas.
vi.mock('./Leaf', () => ({ PANE_FREED: 'devpit:pane-freed', Leaf: () => null }))

afterEach(cleanup)

const session = {
  name: 'api-worker',
  status: 'idle',
  cwd: '/w',
  projectName: 'api',
  cardId: null,
  waiting: null,
  pane: { projectId: 'p', paneId: '%7' },
} as unknown as LiveSession

function freed(): string[] {
  const heard: string[] = []
  window.addEventListener(PANE_FREED, (event) => heard.push((event as CustomEvent<string>).detail))
  return heard
}

describe("a session's terminal over the chat", () => {
  it('hands the pane back to its tab when it is hidden', () => {
    const heard = freed()
    const { unmount } = render(<SessionTerminal session={session} onGo={() => {}} onClose={() => {}} />)
    unmount()
    expect(heard).toEqual(['%7'])
  })

  it('does not hand back a pane it closed', async () => {
    const heard = freed()
    const onClose = vi.fn()
    const { unmount } = render(<SessionTerminal session={session} onGo={() => {}} onClose={onClose} />)
    fireEvent.click(screen.getByText('Close terminal…'))
    fireEvent.click(screen.getByText('Close terminal'))
    await waitFor(() => expect(onClose).toHaveBeenCalled())
    unmount()
    expect(heard).toEqual([])
  })
})
