import { act, cleanup, render, screen } from '@testing-library/react'
import { renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { PaneRunning } from '../gen/bindings'
import type { Tab } from './strip'
import { StopRunning } from './StopRunning'
import { useClosing } from './useClosing'

afterEach(cleanup)

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: { settingsWrite: () => null },
}))

const file = (id: string): Tab => ({ id, kind: 'file', title: 'App.tsx', panes: [] })

const busy: PaneRunning = {
  paneId: 'leaf_1',
  command: 'claude',
  busy: true,
  agent: 'claude',
  label: 'Claude Code',
}

const guard = (over: {
  unsaved?: string[]
  running?: PaneRunning[]
  open?: Tab[]
  closeNow?: (id: string) => void
}) =>
  renderHook(() =>
    useClosing({
      open: over.open ?? [file('t1')],
      closeNow: over.closeNow ?? (() => {}),
      running: over.running ?? [],
      unsaved: new Set(over.unsaved ?? []),
    }),
  )

/*
 * Closing a file that was edited and never saved.
 *
 * The cross is drawn by the strip and the edit lives in the pane, so neither
 * can see the other. The pane reports; this is what the strip's close then
 * does with it.
 */
describe('closing a file with unsaved changes', () => {
  it('stops to ask rather than throwing the edit away', () => {
    const closed = vi.fn()
    const { result } = guard({ unsaved: ['t1'], closeNow: closed })
    act(() => result.current.close('t1'))
    expect(closed).not.toHaveBeenCalled()
    expect(result.current.closing?.stops.kind).toBe('unsaved')
  })

  it('closes a file with nothing to lose without asking', () => {
    const closed = vi.fn()
    const { result } = guard({ closeNow: closed })
    act(() => result.current.close('t1'))
    expect(closed).toHaveBeenCalledWith('t1')
    expect(result.current.closing).toBeNull()
  })

  it('closes once the discard is confirmed', () => {
    const closed = vi.fn()
    const { result } = guard({ unsaved: ['t1'], closeNow: closed })
    act(() => result.current.close('t1'))
    act(() => result.current.confirmClose(false))
    expect(closed).toHaveBeenCalledWith('t1')
  })

  it('leaves the file open when the question is cancelled', () => {
    const closed = vi.fn()
    const { result } = guard({ unsaved: ['t1'], closeNow: closed })
    act(() => result.current.close('t1'))
    act(() => result.current.cancelClose())
    expect(closed).not.toHaveBeenCalled()
    expect(result.current.closing).toBeNull()
  })

  it('keeps asking even when the prompt has been switched off', () => {
    // The setting says "stop asking about running terminals". Reading it as
    // "stop asking before discarding what I typed" would be a tick box that
    // quietly destroys files.
    const closed = vi.fn()
    const { result } = guard({ unsaved: ['t1'], closeNow: closed })
    act(() => result.current.setConfirmStop(false))
    act(() => result.current.close('t1'))
    expect(closed).not.toHaveBeenCalled()
    expect(result.current.closing?.stops.kind).toBe('unsaved')
  })

  it('still lets the prompt be switched off for a running terminal', () => {
    const closed = vi.fn()
    const { result } = guard({ running: [busy], closeNow: closed })
    act(() => result.current.setConfirmStop(false))
    act(() => result.current.close('t1'))
    expect(closed).toHaveBeenCalledWith('t1')
  })
})

describe('what the dialog says', () => {
  const show = (kind: 'unsaved' | 'agent' | 'command') =>
    render(
      <StopRunning
        closing={{ id: 't1', tab: 'App.tsx', stops: { kind, label: 'App.tsx' } }}
        onCancel={() => {}}
        onConfirm={() => {}}
      />,
    )

  it('asks about the changes, not about stopping something', () => {
    show('unsaved')
    expect(screen.getByText('Close without saving?')).toBeTruthy()
    expect(screen.getByText('Discard Changes')).toBeTruthy()
  })

  it('does not offer to stop asking about unsaved work', () => {
    show('unsaved')
    expect(screen.queryByRole('checkbox')).toBeNull()
  })

  it('still offers it for a running terminal', () => {
    show('agent')
    expect(screen.getByRole('checkbox')).toBeTruthy()
    expect(screen.getByText('Stop Agent')).toBeTruthy()
  })

  it('words a command differently from an agent', () => {
    show('command')
    expect(screen.getByText('Stop running command?')).toBeTruthy()
  })
})
