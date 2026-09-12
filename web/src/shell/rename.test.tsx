import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

afterEach(cleanup)

import { Rename } from './Rename'

/* The pure rules are in `strip.test.ts`. This drives the field itself,
   because the two bugs that shipped — a field that never took focus, and a
   double click the browser never reported — were both invisible to a rule. */

describe('renaming in place', () => {
  it('shows the name until it is asked to edit', () => {
    render(<Rename value="Terminal 1" editing={false} onDone={() => {}} />)
    expect(screen.queryByRole('textbox')).toBeNull()
    expect(screen.getByText('Terminal 1')).toBeTruthy()
  })

  it('puts the name in a field that has the focus', async () => {
    render(<Rename value="Terminal 1" editing onDone={() => {}} />)
    const field = screen.getByRole('textbox') as HTMLInputElement
    expect(field.value).toBe('Terminal 1')
    await new Promise((wake) => requestAnimationFrame(wake))
    /* Focused, not merely present: a field nobody focused swallows every
       keystroke and looks like a feature that does not work. */
    expect(document.activeElement).toBe(field)
    expect(field.selectionStart).toBe(0)
    expect(field.selectionEnd).toBe('Terminal 1'.length)
  })

  it('hands back what was typed on Enter', () => {
    const done = vi.fn()
    render(<Rename value="Terminal 1" editing onDone={done} />)
    const field = screen.getByRole('textbox')
    fireEvent.change(field, { target: { value: 'deploy' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    expect(done).toHaveBeenCalledWith('deploy')
  })

  it('hands back nothing on Escape, whatever was typed', () => {
    const done = vi.fn()
    render(<Rename value="Terminal 1" editing onDone={done} />)
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'deploy' } })
    fireEvent.keyDown(screen.getByRole('textbox'), { key: 'Escape' })
    expect(done).toHaveBeenCalledWith(null)
  })

  it('keeps the old name when nothing changed', () => {
    /* Committing an unchanged name would still be a rename in the log. */
    const done = vi.fn()
    render(<Rename value="Terminal 1" editing onDone={done} />)
    fireEvent.keyDown(screen.getByRole('textbox'), { key: 'Enter' })
    expect(done).toHaveBeenCalledWith(null)
  })

  it('commits when you click away', () => {
    const done = vi.fn()
    render(<Rename value="Terminal 1" editing onDone={done} />)
    const field = screen.getByRole('textbox')
    fireEvent.change(field, { target: { value: 'deploy' } })
    fireEvent.blur(field)
    expect(done).toHaveBeenCalledWith('deploy')
  })
})
