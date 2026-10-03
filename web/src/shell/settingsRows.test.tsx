import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { PrefSwitch } from './PrefSwitch'

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

describe('a settings switch', () => {
  it('flips from the switch alone, not from the words beside it', () => {
    const flip = vi.fn()
    render(<PrefSwitch on={false} onFlip={flip} title="Open at login" said="devpit starts with your desktop." more="One copy runs at a time." />)
    fireEvent.click(screen.getByText('Open at login'))
    fireEvent.click(screen.getByText('devpit starts with your desktop.'))
    fireEvent.click(screen.getByText('How it works'))
    expect(flip).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('switch', { name: 'Open at login' }))
    expect(flip).toHaveBeenCalledOnce()
  })

  it('keeps the long half folded until it is asked for', () => {
    render(<PrefSwitch on onFlip={vi.fn()} title="Island" said="A small window." more="Drag it to another screen." />)
    expect(screen.getByText('Drag it to another screen.').closest('details')?.open).toBe(false)
  })
})
