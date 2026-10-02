import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { DeskSettings, keysOf } from './DeskSettings'

const set = vi.fn(async (keys: string | null) => keys)
let atLogin = false
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    atLoginRead: async () => atLogin,
    atLoginSet: async (on: boolean) => ((atLogin = on), on),
    shortcutRead: async () => null,
    shortcutSet: (keys: string | null) => set(keys),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  atLogin = false
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
})
