import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { DocPeek } from './DocPeek'
import { PEEK_DEFAULT } from './peekWidth'

vi.mock('./useFile', () => ({ useFile: () => ({ file: { text: '# Plan' }, error: null }) }))
vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))

afterEach(() => {
  cleanup()
  localStorage.clear()
})

const inPane = () => {
  const pane = document.createElement('div')
  Object.defineProperty(pane, 'clientWidth', { value: 1400 })
  document.body.appendChild(pane)
  return { pane, ...render(<DocPeek path="docs/plan.md" onOpen={vi.fn()} onClose={vi.fn()} />, { container: pane }) }
}

describe('a document read beside the chat', () => {
  it('makes room in the pane for itself, and gives it back when it closes', () => {
    localStorage.setItem('devpit:peek-width', '700')
    const { pane, unmount } = inPane()
    expect(pane.style.getPropertyValue('--peek-w')).toBe('700px')
    unmount()
    expect(pane.style.getPropertyValue('--peek-w')).toBe('')
  })

  it('goes back to its default width on a double click of its edge', () => {
    localStorage.setItem('devpit:peek-width', '900')
    const { pane } = inPane()
    fireEvent.doubleClick(screen.getByRole('separator'))
    expect(pane.style.getPropertyValue('--peek-w')).toBe(`${PEEK_DEFAULT}px`)
    expect(localStorage.getItem('devpit:peek-width')).toBe(String(PEEK_DEFAULT))
  })
})
