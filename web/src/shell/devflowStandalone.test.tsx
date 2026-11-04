import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { DevflowBinary, DevflowPlugin, DevflowPluginAction } from '../gen/bindings'

const missing: DevflowPlugin = {
  projectId: 'p1',
  directory: '/h/.claude-work',
  profile: 'Work',
  version: null,
  enabled: false,
  known: '0.2.1',
  twice: false,
}
let plugin: DevflowPlugin = missing
let binary: DevflowBinary = {
  path: '/h/.local/bin/devflow',
  version: null,
  known: '0.2.1',
  doctor: [],
}
let offline = false
const called = vi.fn()

vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => {
    try {
      return { data: await call(), error: null, loading: false }
    } catch (err) {
      return {
        data: null,
        error: String((err as Error).message),
        loading: false,
      }
    }
  },
  commands: {
    devflowPlugin: async () => plugin,
    devflowPluginDo: async (projectId: string, action: DevflowPluginAction) => {
      called(projectId, action)
      if (action === 'install') plugin = { ...plugin, version: '0.2.1', enabled: true }
      if (action === 'remove') plugin = { ...plugin, version: null, enabled: false, twice: false }
      if (action === 'disable') plugin = { ...plugin, enabled: false, twice: false }
      return plugin
    },
    devflowBinary: async () => binary,
    devflowBinaryInstall: async () => {
      called('binary')
      if (offline) throw new Error('devflow could not be downloaded: is this machine online?')
      binary = {
        ...binary,
        version: '0.2.1',
        doctor: ['ok  devflow is on PATH'],
      }
      return binary
    },
  },
}))

const { DevflowBinaryRow, DevflowPluginRow } = await import('./DevflowStandalone')

afterEach(() => {
  cleanup()
  called.mockReset()
  plugin = missing
  binary = {
    path: '/h/.local/bin/devflow',
    version: null,
    known: '0.2.1',
    doctor: [],
  }
  offline = false
})

describe('the devflow plugin button', () => {
  it('installs only after a yes to what it runs in the profile, then updates and removes it', async () => {
    render(<DevflowPluginRow projectId="p1" devpitOn={false} onDeactivate={() => {}} />)
    fireEvent.click(await screen.findByRole('button', { name: 'Install plugin…' }))
    expect(called).not.toHaveBeenCalled()
    expect(screen.getByText(/claude plugin install devflow@devflow/).closest('.pref__d')?.textContent).toContain('CLAUDE_CONFIG_DIR at /h/.claude-work')
    fireEvent.click(screen.getByRole('button', { name: 'Install' }))
    expect(await screen.findByText(/devflow@devflow 0\.2\.1, on in Work/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Update' }))
    await waitFor(() => expect(called).toHaveBeenCalledWith('p1', 'update'))
    fireEvent.click(screen.getByRole('button', { name: 'Remove' }))
    expect(await screen.findByRole('button', { name: 'Install plugin…' })).toBeTruthy()
    expect(called.mock.calls).toEqual([
      ['p1', 'install'],
      ['p1', 'update'],
      ['p1', 'remove'],
    ])
  })

  it("says when the plugin and devpit's Devflow are both on, and switches either off", async () => {
    plugin = { ...missing, version: '0.2.1', enabled: true, twice: true }
    const deactivate = vi.fn()
    render(<DevflowPluginRow projectId="p1" devpitOn onDeactivate={deactivate} />)
    expect((await screen.findByRole('alert')).textContent).toContain('The devflow tools twice')
    fireEvent.click(screen.getByRole('button', { name: 'Deactivate devpit’s Devflow' }))
    expect(deactivate).toHaveBeenCalledOnce()
    fireEvent.click(screen.getByRole('button', { name: 'Disable the plugin' }))
    await waitFor(() => expect(screen.queryByRole('alert')).toBeNull())
    expect(called).toHaveBeenCalledWith('p1', 'disable')
  })
})

describe('the devflow binary button', () => {
  it('installs after a yes, then shows the version, doctor and Update', async () => {
    render(<DevflowBinaryRow />)
    fireEvent.click(await screen.findByRole('button', { name: 'Install devflow…' }))
    expect(screen.getByText(/checksums\.txt/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Install' }))
    expect(await screen.findByText(/devflow 0\.2\.1 at \/h\/\.local\/bin\/devflow/)).toBeTruthy()
    expect(screen.getByLabelText('devflow doctor').textContent).toBe('ok  devflow is on PATH')
    expect(screen.getByRole('button', { name: 'Update to 0.2.1…' })).toBeTruthy()
  })

  it('says why when there is no network, and stays not installed', async () => {
    offline = true
    render(<DevflowBinaryRow />)
    fireEvent.click(await screen.findByRole('button', { name: 'Install devflow…' }))
    fireEvent.click(screen.getByRole('button', { name: 'Install' }))
    expect(await screen.findByText(/could not be downloaded/)).toBeTruthy()
    expect(screen.getByText(/Not installed at/)).toBeTruthy()
  })
})
