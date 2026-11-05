import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Display } from '../gen/bindings'
import { DisplaySettings } from './DisplaySettings'

let display: Display = { offered: true, running: 'wayland', chosen: null, why: 'default' }
const set = vi.fn(async (backend: Display['chosen']) => (display = { ...display, chosen: backend, why: 'chosen' }))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    displayRead: async () => display,
    displaySet: (backend: Display['chosen']) => set(backend),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  display = { offered: true, running: 'wayland', chosen: null, why: 'default' }
})

describe('the display setting', () => {
  it('says what this start runs on and why', async () => {
    display = { ...display, running: 'x11', why: 'failedBefore' }
    render(<DisplaySettings />)
    expect(await screen.findByText(/On X11 \(XWayland\)\. The last Wayland start never loaded its window/)).toBeTruthy()
    expect(screen.getByRole('radio', { name: 'Automatic' }).getAttribute('aria-checked')).toBe('true')
  })

  it('records the choice for the next start', async () => {
    render(<DisplaySettings />)
    fireEvent.click(await screen.findByRole('radio', { name: 'X11' }))
    await waitFor(() => expect(set).toHaveBeenCalledWith('x11'))
    await waitFor(() => expect(screen.getByRole('radio', { name: 'X11' }).getAttribute('aria-checked')).toBe('true'))
  })

  it('is not offered where there is no choice', async () => {
    display = { ...display, offered: false }
    const { container } = render(<DisplaySettings />)
    await waitFor(() => expect(container.innerHTML).toBe(''))
  })
})
