import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { PluginFile, PluginList, PluginManifest } from '../gen/bindings'
import { DRAWINGS } from '../plugins/excalidraw/drawings'
import { PluginsPane } from './PluginsPane'
import type { Tab } from './strip'
import { PluginsProvider } from './usePlugins'

afterEach(cleanup)

const excalidraw: PluginManifest = {
  id: 'excalidraw',
  name: 'Excalidraw',
  version: '0.18.1',
  description: 'Sketches kept as files in the project.',
  surfaces: [{ type: 'pane', many: true }, { type: 'cardPin' }],
  data: { extensions: ['.excalidraw'], maxBytes: 33554432 },
  permissions: ['dataOwn'],
}

/* The backend, as far as these tests need one: it remembers what was installed and switched. */
let stored: Record<string, { installed: boolean; enabled: boolean }> = {}
let files: PluginFile[] | null = []
let unreadable = false
let pending = false
let failing: string | null = null
const switched = vi.fn()
const installed = vi.fn()
const uninstalled = vi.fn()
const listed = vi.fn()
const deleted = vi.fn()
const catalogue = (): PluginList => ({
  plugins: [{ manifest: excalidraw, ...(stored.excalidraw ?? { installed: false, enabled: false }) }],
})

vi.mock('./live', () => ({
  ask: (call: () => unknown) =>
    Promise.resolve(call()).then((data) =>
      failing ? { data: null, error: failing, loading: false } : { data, error: null, loading: false },
    ),
  commands: {
    pluginList: () => {
      listed()
      if (pending) return new Promise(() => {})
      return unreadable ? null : catalogue()
    },
    pluginDataList: () => (files === null ? null : { files }),
    pluginDataDelete: (...args: unknown[]) => deleted(...args),
    pluginSetEnabled: (projectId: string, pluginId: string, enabled: boolean) => {
      switched(projectId, pluginId, enabled)
      stored = { ...stored, [pluginId]: { installed: true, enabled } }
      return catalogue()
    },
    pluginInstall: (projectId: string, pluginId: string) => {
      installed(projectId, pluginId)
      stored = { ...stored, [pluginId]: { installed: true, enabled: true } }
      return catalogue()
    },
    pluginUninstall: (projectId: string, pluginId: string, deleteData: boolean) => {
      uninstalled(projectId, pluginId, deleteData)
      stored = { ...stored, [pluginId]: { installed: false, enabled: false } }
      return { ...catalogue(), removedFiles: deleteData ? (files?.length ?? 0) : 0 }
    },
  },
}))

const shell = {
  project: { id: 'p', name: 'demo' } as { id: string; name: string } | null,
  close: vi.fn(),
  show: vi.fn(),
  open: [] as Tab[],
  closeNow: vi.fn(),
}
vi.mock('./useShell', () => ({ useShell: () => shell }))

const file = (name: string): PluginFile => ({ name, bytes: 10, modified: null })

beforeEach(() => {
  stored = { excalidraw: { installed: true, enabled: false } }
  files = []
  unreadable = false
  pending = false
  failing = null
  for (const spy of [switched, installed, uninstalled, listed, deleted]) spy.mockClear()
  shell.closeNow.mockClear()
  shell.show.mockClear()
  shell.open = []
  shell.project = { id: 'p', name: 'demo' }
})

const checked = (): string | null => screen.getByRole('switch').getAttribute('aria-checked')
const labels = (): (string | null)[] => screen.getAllByRole('button').map((button) => button.getAttribute('aria-label'))
const dialog = (): HTMLElement => screen.getByRole('dialog')
const press = (name: string | RegExp): void => {
  fireEvent.click(within(dialog()).getByRole('button', { name }))
}

const pane = () =>
  render(
    <PluginsProvider>
      <PluginsPane />
    </PluginsProvider>,
  )

describe('the Capabilities pane', () => {
  it('lists the catalogue with name, version and description', async () => {
    pane()
    expect(await screen.findByText('Excalidraw')).toBeTruthy()
    expect(screen.getByText('0.18.1')).toBeTruthy()
    expect(screen.getByText('Sketches kept as files in the project.')).toBeTruthy()
  })

  it('offers nothing to browse, and no control beyond the ones a card can act on', async () => {
    pane()
    await screen.findByRole('switch')
    expect(screen.queryByText(/available|browse/i)).toBeNull()
    /* Installed: its switch, and Uninstall. Open only comes with on. */
    expect(labels()).toEqual(['Close Capabilities', 'Uninstall Excalidraw'])
    expect(screen.getAllByRole('switch')).toHaveLength(1)
  })

  it('says in its header how many are installed and how many are on', async () => {
    stored = {}
    pane()
    expect(await screen.findByText(/· 0 installed · 0 on/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Install Excalidraw' }))
    press('Install')
    expect(await screen.findByText(/· 1 installed · 1 on/)).toBeTruthy()
    fireEvent.click(screen.getByRole('switch'))
    expect(await screen.findByText(/· 1 installed · 0 on/)).toBeTruthy()
  })

  it('draws each capability as a card saying what it adds in plain words', async () => {
    pane()
    const card = await screen.findByRole('article', { name: 'Excalidraw' })
    expect(within(card).getByText('Opens beside your work')).toBeTruthy()
    expect(within(card).getByText('Pins to cards')).toBeTruthy()
    expect(card.textContent).not.toMatch(/\bpane\b/i)
  })

  it('turns a capability on through the backend', async () => {
    pane()
    fireEvent.click(await screen.findByRole('switch'))
    await waitFor(() => expect(checked()).toBe('true'))
    expect(switched).toHaveBeenCalledWith('p', 'excalidraw', true)
  })

  it('switches only from the switch, never from the rest of the card', async () => {
    pane()
    const card = await screen.findByRole('article', { name: 'Excalidraw' })
    fireEvent.click(card)
    fireEvent.click(within(card).getByText('Excalidraw'))
    fireEvent.click(within(card).getByText('Sketches kept as files in the project.'))
    await act(async () => {})
    expect(switched).not.toHaveBeenCalled()
    expect(checked()).toBe('false')
  })

  it('offers Open only while on, and Open lands on the capability’s surface', async () => {
    pane()
    await screen.findByRole('switch')
    expect(screen.queryByRole('button', { name: 'Open Excalidraw' })).toBeNull()

    fireEvent.click(screen.getByRole('switch'))
    fireEvent.click(await screen.findByRole('button', { name: 'Open Excalidraw' }))
    expect(shell.show).toHaveBeenCalledWith('drawing', DRAWINGS)

    fireEvent.click(screen.getByRole('switch'))
    await waitFor(() => expect(checked()).toBe('false'))
    expect(screen.queryByRole('button', { name: 'Open Excalidraw' })).toBeNull()
  })

  it('keeps the switch where it was left when the pane is closed and opened again', async () => {
    const { rerender } = pane()
    fireEvent.click(await screen.findByRole('switch'))
    await waitFor(() => expect(checked()).toBe('true'))

    rerender(<PluginsProvider>{null}</PluginsProvider>)
    expect(screen.queryByRole('switch')).toBeNull()

    rerender(
      <PluginsProvider>
        <PluginsPane />
      </PluginsProvider>,
    )
    expect(checked()).toBe('true')
  })

  it('reads the switch back from the backend when the window starts again', async () => {
    const first = pane()
    fireEvent.click(await screen.findByRole('switch'))
    await waitFor(() => expect(checked()).toBe('true'))
    first.unmount()

    pane()
    await waitFor(() => expect(checked()).toBe('true'))
  })
})

describe('installing a capability', () => {
  beforeEach(() => {
    stored = {}
  })

  it('offers Install and no switch, Open or Uninstall while it is not installed', async () => {
    pane()
    const card = await screen.findByRole('article', { name: 'Excalidraw' })
    expect(labels()).toEqual(['Close Capabilities', 'Install Excalidraw'])
    expect(screen.queryByRole('switch')).toBeNull()
    expect(within(card).getByText('Not installed')).toBeTruthy()
  })

  it('asks first, naming what it adds and where its files live, and cancel installs nothing', async () => {
    pane()
    fireEvent.click(await screen.findByRole('button', { name: 'Install Excalidraw' }))
    expect(within(dialog()).getByText('Install Excalidraw?')).toBeTruthy()
    expect(within(dialog()).getByText('Opens beside your work')).toBeTruthy()
    expect(dialog().textContent).toMatch(/this project’s folder/)

    press('Cancel')
    await act(async () => {})
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(installed).not.toHaveBeenCalled()
    expect(screen.queryByRole('switch')).toBeNull()
  })

  it('installs once on confirm, and draws it on as the backend answered', async () => {
    pane()
    fireEvent.click(await screen.findByRole('button', { name: 'Install Excalidraw' }))
    press('Install')
    await waitFor(() => expect(checked()).toBe('true'))
    expect(installed).toHaveBeenCalledTimes(1)
    expect(installed).toHaveBeenCalledWith('p', 'excalidraw')
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(screen.getByRole('button', { name: 'Open Excalidraw' })).toBeTruthy()
  })
})

describe('uninstalling a capability', () => {
  const list: Tab = { id: 'drawings', kind: 'drawing', title: 'Excalidraw' }
  const fluxo: Tab = { id: 'drawing:fluxo.excalidraw', kind: 'drawing', path: 'fluxo.excalidraw', title: 'fluxo' }
  const terminal: Tab = { id: 'term_1', kind: 'term', title: 'Terminal 1' }

  const begin = async (): Promise<void> => {
    fireEvent.click(await screen.findByRole('button', { name: 'Uninstall Excalidraw' }))
    expect(within(dialog()).getByText('Uninstall Excalidraw?')).toBeTruthy()
  }

  it('takes two steps, and leaving at either one uninstalls nothing', async () => {
    pane()
    await begin()
    press('Cancel')

    await begin()
    press('Continue')
    expect(within(dialog()).getByRole('checkbox')).toBeTruthy()
    press('Cancel')

    await begin()
    press('Continue')
    fireEvent.click(document.querySelector('.ask')!)

    await act(async () => {})
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(uninstalled).not.toHaveBeenCalled()
    expect(screen.getByRole('switch')).toBeTruthy()
  })

  it('keeps the data unless the box is ticked, and the box starts unticked every time', async () => {
    files = [file('a.excalidraw')]
    pane()
    await begin()
    press('Continue')
    fireEvent.click(within(dialog()).getByRole('checkbox'))
    press('Cancel')

    await begin()
    press('Continue')
    expect(within(dialog()).getByRole('checkbox').getAttribute('aria-checked')).toBe('false')
    press('Uninstall')
    await waitFor(() => expect(uninstalled).toHaveBeenCalledWith('p', 'excalidraw', false))
    expect(await screen.findByText(/Its files stay/)).toBeTruthy()
  })

  it('names the files it will delete on the final button, and deletes them when ticked', async () => {
    files = [file('a.excalidraw'), file('b.excalidraw'), file('c.excalidraw')]
    pane()
    await begin()
    press('Continue')
    await waitFor(() => expect(within(dialog()).getByText(/3 files in this project’s folder/)).toBeTruthy())
    expect(within(dialog()).getByRole('button', { name: 'Uninstall' })).toBeTruthy()

    fireEvent.click(within(dialog()).getByRole('checkbox'))
    press('Uninstall and delete 3 files')
    await waitFor(() => expect(uninstalled).toHaveBeenCalledWith('p', 'excalidraw', true))
    expect(uninstalled).toHaveBeenCalledTimes(1)
    expect(await screen.findByRole('status')).toHaveProperty('textContent', 'Excalidraw uninstalled and 3 files deleted.')
  })

  it('does not claim a count it could not read', async () => {
    files = null
    pane()
    await begin()
    press('Continue')
    fireEvent.click(within(dialog()).getByRole('checkbox'))
    expect(within(dialog()).getByRole('button', { name: 'Uninstall and delete its data' })).toBeTruthy()
  })

  it('closes its tabs and returns the card to Install', async () => {
    stored = { excalidraw: { installed: true, enabled: true } }
    shell.open = [list, fluxo, terminal]
    pane()
    await waitFor(() => expect(checked()).toBe('true'))
    await begin()
    press('Continue')
    press('Uninstall')

    await waitFor(() => expect(shell.closeNow).toHaveBeenCalledWith(fluxo.id))
    expect(shell.closeNow).toHaveBeenCalledWith(list.id)
    expect(shell.closeNow).not.toHaveBeenCalledWith(terminal.id)
    expect(labels()).toEqual(['Close Capabilities', 'Install Excalidraw'])
    expect(screen.queryByRole('switch')).toBeNull()
  })
})

describe('the Capabilities pane before it has cards to draw', () => {
  it('says there is no project, and draws no card', async () => {
    shell.project = null
    pane()
    expect(await screen.findByText('No project open.')).toBeTruthy()
    expect(screen.queryByRole('switch')).toBeNull()
  })

  it('keeps the shape of a card while the catalogue is read', async () => {
    pending = true
    pane()
    await waitFor(() => expect(listed).toHaveBeenCalled())
    expect(screen.getByLabelText('Reading capabilities').getAttribute('aria-busy')).toBe('true')
    expect(screen.queryByRole('switch')).toBeNull()
  })

  it('says why the catalogue could not be read', async () => {
    failing = 'the store is locked'
    pane()
    expect(await screen.findByText('the store is locked')).toBeTruthy()
    expect(screen.queryByLabelText('Reading capabilities')).toBeNull()
    expect(screen.queryByRole('switch')).toBeNull()
  })
})

describe('switching a capability off', () => {
  const list: Tab = { id: 'drawings', kind: 'drawing', title: 'Excalidraw' }
  const fluxo: Tab = { id: 'drawing:fluxo.excalidraw', kind: 'drawing', path: 'fluxo.excalidraw', title: 'fluxo' }
  const terminal: Tab = { id: 'term_1', kind: 'term', title: 'Terminal 1' }

  it('closes its tabs and leaves its files', async () => {
    stored = { excalidraw: { installed: true, enabled: true } }
    shell.open = [list, fluxo, terminal]
    pane()
    await waitFor(() => expect(checked()).toBe('true'))
    expect(shell.closeNow).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole('switch'))
    await waitFor(() => expect(shell.closeNow).toHaveBeenCalledWith(fluxo.id))
    expect(shell.closeNow).toHaveBeenCalledWith(list.id)
    expect(shell.closeNow).not.toHaveBeenCalledWith(terminal.id)
    expect(deleted).not.toHaveBeenCalled()
  })

  it('closes the drawing tabs a project restored while the capability was off', async () => {
    shell.open = [fluxo, terminal]
    render(<PluginsProvider>{null}</PluginsProvider>)
    await waitFor(() => expect(shell.closeNow).toHaveBeenCalledWith(fluxo.id))
    expect(shell.closeNow).not.toHaveBeenCalledWith(terminal.id)
  })

  it('closes nothing when the catalogue could not be read', async () => {
    unreadable = true
    shell.open = [fluxo]
    render(<PluginsProvider>{null}</PluginsProvider>)
    await waitFor(() => expect(listed).toHaveBeenCalled())
    await act(async () => {})
    expect(shell.closeNow).not.toHaveBeenCalled()
  })
})
