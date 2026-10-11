import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { DeskSettings, keysOf } from './DeskSettings'

const set = vi.fn(async (keys: string | null) => keys)
let atLogin = false
let tray = { shown: true, keep: true }
const keep = vi.fn(async (on: boolean) => (tray = { ...tray, keep: on }))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    atLoginRead: async () => atLogin,
    atLoginSet: async (on: boolean) => ((atLogin = on), on),
    shortcutRead: async () => null,
    shortcutSet: (keys: string | null) => set(keys),
    trayRead: async () => tray,
    trayKeepSet: (on: boolean) => keep(on),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  atLogin = false
  tray = { shown: true, keep: true }
})

const press = (over: Partial<KeyboardEvent>) =>
  keysOf({ key: 'd', code: 'KeyD', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...over })

describe('the keys of a shortcut', () => {
  it('are spelled as the shortcut plugin reads them', () => {
    expect(press({ ctrlKey: true, shiftKey: true })).toBe('CommandOrControl+Shift+D')
    expect(press({ metaKey: true, altKey: true, code: 'Digit1', key: '1' })).toBe('CommandOrControl+Alt+1')
    expect(press({ altKey: true, code: 'Space', key: ' ' })).toBe('Alt+Space')
  })

  it('are never one bare key, nor a modifier alone', () => {
    expect(press({})).toBeNull()
    expect(press({ key: 'Shift', code: 'ShiftLeft', shiftKey: true })).toBeNull()
  })
})

describe('devpit as a desktop app', () => {
  it('records a shortcut from the next keys pressed', async () => {
    render(<DeskSettings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Choose' }))
    fireEvent.keyDown(window, { key: 'd', code: 'KeyD', ctrlKey: true, altKey: true })
    await waitFor(() => expect(set).toHaveBeenCalledWith('CommandOrControl+Alt+D'))
    await waitFor(() => screen.getByText(/CommandOrControl\+Alt\+D brings devpit forward/))
  })

  it('opens at login when switched on', async () => {
    render(<DeskSettings />)
    const login = await screen.findByRole('switch', { name: /Open at login/ })
    fireEvent.click(login)
    await waitFor(() => expect(atLogin).toBe(true))
  })

  it('keeps devpit in the tray until the switch is turned off', async () => {
    render(<DeskSettings />)
    const kept = await screen.findByRole('switch', { name: /Keep devpit in the tray/ })
    await waitFor(() => expect(kept.getAttribute('aria-checked')).toBe('true'))
    fireEvent.click(kept)
    await waitFor(() => expect(keep).toHaveBeenCalledWith(false))
    await waitFor(() => expect(kept.getAttribute('aria-checked')).toBe('false'))
  })

  it('offers no switch where no tray shows, and says closing quits', async () => {
    tray = { shown: false, keep: true }
    render(<DeskSettings />)
    await screen.findByText(/shows no tray icon, so closing the window quits devpit/)
    expect(screen.queryByRole('switch', { name: /Keep devpit in the tray/ })).toBeNull()
  })
})
