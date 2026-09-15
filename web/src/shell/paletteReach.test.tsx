import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { PluginList, PluginManifest } from '../gen/bindings'
import { DRAWINGS } from '../plugins/excalidraw/drawings'
import { useReachable } from './paletteReach'
import { PluginsProvider } from './usePlugins'

afterEach(cleanup)

const excalidraw: PluginManifest = {
  id: 'excalidraw',
  name: 'Excalidraw',
  version: '0.18.1',
  description: '',
  surfaces: [{ type: 'pane', many: true }],
  data: { extensions: ['.excalidraw'], maxBytes: 33554432 },
  permissions: ['dataOwn'],
}

let catalogue: PluginList = { plugins: [] }
const listed = vi.fn()

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve(call()).then((data) => ({ data, error: null, loading: false })),
  commands: {
    pluginList: () => {
      listed()
      return catalogue
    },
    agentsKnown: () => [],
    projectFiles: () => ({ paths: [], partial: false }),
    boardGet: () => ({ cards: [] }),
  },
}))

const shell = {
  project: { id: 'p', name: 'demo' },
  open: [],
  show: vi.fn(),
  openPrefs: vi.fn(),
  focus: vi.fn(),
  closeNow: vi.fn(),
}
vi.mock('./useShell', () => ({ useShell: () => shell }))

beforeEach(() => {
  listed.mockClear()
  shell.show.mockClear()
})

const reach = () =>
  renderHook(() => useReachable(), {
    wrapper: ({ children }: { children: React.ReactNode }) => <PluginsProvider>{children}</PluginsProvider>,
  })

const keys = (rows: readonly { key: string }[]): string[] => rows.map((row) => row.key)

describe('the palette reaches panes', () => {
  it('offers the Capabilities pane like the other panes, by that name', async () => {
    catalogue = { plugins: [{ manifest: excalidraw, installed: true, enabled: false }] }
    const { result } = reach()
    expect(keys(result.current.panes)).toContain('pane:board')
    expect(result.current.panes.find((row) => row.key === 'pane:plugins')?.name).toBe('Capabilities')
    await waitFor(() => expect(listed).toHaveBeenCalled())
  })

  it('offers Excalidraw while it is on, landing on the one list tab', async () => {
    catalogue = { plugins: [{ manifest: excalidraw, installed: true, enabled: true }] }
    const { result } = reach()
    await waitFor(() => expect(keys(result.current.panes)).toContain('pane:drawing'))
    const row = result.current.panes.find((one) => one.key === 'pane:drawing')!
    expect(row.name).toBe('Excalidraw')
    row.go()
    expect(shell.show).toHaveBeenCalledWith('drawing', DRAWINGS)
  })

  it('names that row as the capability’s manifest does', async () => {
    /* A name nothing in the app spells, so the row can only have read it. */
    catalogue = { plugins: [{ manifest: { ...excalidraw, name: 'Whiteboard' }, installed: true, enabled: true }] }
    const { result } = reach()
    await waitFor(() => expect(keys(result.current.panes)).toContain('pane:drawing'))
    expect(result.current.panes.find((row) => row.key === 'pane:drawing')?.name).toBe('Whiteboard')
  })

  it('does not offer Excalidraw while it is off', async () => {
    catalogue = { plugins: [{ manifest: excalidraw, installed: true, enabled: false }] }
    const { result } = reach()
    await waitFor(() => expect(listed).toHaveBeenCalled())
    await act(async () => {})
    expect(keys(result.current.panes)).not.toContain('pane:drawing')
  })

  it('does not offer Excalidraw while it is not installed, whatever its switch says', async () => {
    catalogue = { plugins: [{ manifest: excalidraw, installed: false, enabled: true }] }
    const { result } = reach()
    await waitFor(() => expect(listed).toHaveBeenCalled())
    await act(async () => {})
    expect(keys(result.current.panes)).not.toContain('pane:drawing')
  })
})
