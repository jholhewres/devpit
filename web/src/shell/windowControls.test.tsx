import { act, cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { useMaximized } from './useMaximized'
import { WindowControls } from './WindowControls'

let filling = false
let resized: (() => void) | null = null
vi.mock('./window', () => ({
  isMaximized: async () => filling,
  onResized: (then: () => void) => {
    resized = then
    return () => (resized = null)
  },
  minimize: () => {},
  toggleMaximize: () => {},
  close: () => {},
}))

afterEach(() => {
  cleanup()
  filling = false
  resized = null
})

/** The shapes the middle control draws: one square, or two overlapping. */
const shapes = (button: HTMLElement): number => button.querySelectorAll('rect, path').length

function Bar(): React.JSX.Element {
  return <WindowControls maximized={useMaximized()} />
}

describe('the maximize control', () => {
  it('offers to maximize a window that does not fill the screen', () => {
    render(<WindowControls maximized={false} />)
    const button = screen.getByRole('button', { name: 'Maximize' })
    expect(button.dataset.tip).toBe('Maximize')
    expect(shapes(button)).toBe(1)
    expect(screen.queryByRole('button', { name: 'Restore' })).toBeNull()
  })

  it('offers to restore a maximized one, drawn as two squares', () => {
    render(<WindowControls maximized />)
    const button = screen.getByRole('button', { name: 'Restore' })
    expect(button.dataset.tip).toBe('Restore')
    expect(shapes(button)).toBe(2)
    expect(screen.queryByRole('button', { name: 'Maximize' })).toBeNull()
  })

  it('turns into restore and back as the window is resized', async () => {
    render(<Bar />)
    await act(async () => {})
    expect(screen.getByRole('button', { name: 'Maximize' })).toBeTruthy()

    filling = true
    await act(async () => resized?.())
    expect(screen.getByRole('button', { name: 'Restore' })).toBeTruthy()

    filling = false
    await act(async () => resized?.())
    expect(screen.getByRole('button', { name: 'Maximize' })).toBeTruthy()
  })

  it('names every window control, so none is an unlabelled icon', () => {
    render(<WindowControls maximized={false} />)
    for (const button of screen.getAllByRole('button')) {
      expect(button.getAttribute('aria-label')).toBeTruthy()
      expect(button.dataset.tip).toBe(button.getAttribute('aria-label'))
    }
  })
})
