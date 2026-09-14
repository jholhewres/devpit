import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { Grip } from './Grip'
import { CONTENT_LEAST, LEAST, MOST, WIDE } from './sizing'

afterEach(cleanup)

beforeEach(() => {
  Object.defineProperty(window, 'innerWidth', { value: 4000, configurable: true })
  // jsdom has no pointer capture; the drag does not depend on it working,
  // only on it not throwing.
  Element.prototype.setPointerCapture = () => {}
})

/** Renders an edge and reports what it said while dragging and on the drop. */
function edge(over: { panel?: 'sidebar' | 'files'; width?: number; other?: number } = {}) {
  const panel = over.panel ?? 'sidebar'
  const sized = vi.fn()
  const done = vi.fn()
  render(
    <Grip
      panel={panel}
      width={over.width ?? WIDE[panel]}
      other={over.other ?? 0}
      onSize={sized}
      onDone={done}
    />,
  )
  return { grip: screen.getByRole('separator'), sized, done }
}

const drag = (grip: HTMLElement, from: number, to: number): void => {
  fireEvent.pointerDown(grip, { button: 0, clientX: from, pointerId: 1 })
  fireEvent.pointerMove(grip, { clientX: to, pointerId: 1 })
}

describe('dragging an edge', () => {
  it('widens the sidebar as the pointer goes right', () => {
    const { grip, sized } = edge({ width: 252 })
    drag(grip, 252, 352)
    expect(sized).toHaveBeenLastCalledWith(352)
  })

  it('widens the files panel as the pointer goes left', () => {
    const { grip, sized } = edge({ panel: 'files', width: 340 })
    drag(grip, 3660, 3560)
    expect(sized).toHaveBeenLastCalledWith(440)
  })

  it('stops at the panel\'s own limits rather than following the pointer', () => {
    const { grip, sized } = edge({ width: 252 })
    drag(grip, 252, 9000)
    expect(sized).toHaveBeenLastCalledWith(MOST.sidebar)
    drag(grip, 252, -9000)
    expect(sized).toHaveBeenLastCalledWith(LEAST.sidebar)
  })

  it('stops where the work would start disappearing', () => {
    Object.defineProperty(window, 'innerWidth', { value: 900, configurable: true })
    const { grip, sized } = edge({ width: 252, other: 340 })
    drag(grip, 252, 800)
    expect(sized).toHaveBeenLastCalledWith(900 - 340 - CONTENT_LEAST)
  })

  it('writes nothing until the edge is let go', () => {
    const { grip, done } = edge()
    drag(grip, 252, 352)
    expect(done).not.toHaveBeenCalled()
    fireEvent.lostPointerCapture(grip, { pointerId: 1 })
    expect(done).toHaveBeenCalledTimes(1)
  })

  it('ignores a button that is not the first one', () => {
    const { grip, sized } = edge()
    fireEvent.pointerDown(grip, { button: 2, pointerId: 1 })
    fireEvent.pointerMove(grip, { clientX: 900, pointerId: 1 })
    expect(sized).not.toHaveBeenCalled()
  })

  it('ignores a move that no press started', () => {
    const { grip, sized } = edge()
    fireEvent.pointerMove(grip, { clientX: 900, pointerId: 1 })
    expect(sized).not.toHaveBeenCalled()
  })
})

describe('the keyboard, because four pixels is not a target for everyone', () => {
  it('moves the edge with the arrows', () => {
    const { grip, done } = edge({ width: 252 })
    fireEvent.keyDown(grip, { key: 'ArrowRight' })
    expect(done).toHaveBeenLastCalledWith(260)
    fireEvent.keyDown(grip, { key: 'ArrowLeft' })
    expect(done).toHaveBeenLastCalledWith(244)
  })

  it('goes the other way for the panel on the other side', () => {
    const { grip, done } = edge({ panel: 'files', width: 340 })
    fireEvent.keyDown(grip, { key: 'ArrowLeft' })
    expect(done).toHaveBeenLastCalledWith(348)
  })

  it('takes a bigger step with shift', () => {
    const { grip, done } = edge({ width: 252 })
    fireEvent.keyDown(grip, { key: 'ArrowRight', shiftKey: true })
    expect(done).toHaveBeenLastCalledWith(300)
  })

  it('leaves the keys it does not use alone', () => {
    const { grip, done } = edge()
    const event = new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true, cancelable: true })
    fireEvent(grip, event)
    expect(done).not.toHaveBeenCalled()
    expect(event.defaultPrevented).toBe(false)
  })

  it('is reachable by tabbing to it, and says how wide it is', () => {
    const { grip } = edge({ width: 300 })
    expect(grip.getAttribute('tabindex')).toBe('0')
    expect(grip.getAttribute('aria-valuenow')).toBe('300')
    expect(grip.getAttribute('aria-valuemin')).toBe(String(LEAST.sidebar))
    expect(grip.getAttribute('aria-valuemax')).toBe(String(MOST.sidebar))
    expect(grip.getAttribute('aria-label')).toBeTruthy()
  })
})

describe('double click', () => {
  it('puts the panel back to the width it ships with', () => {
    const { grip, done } = edge({ width: 470 })
    fireEvent.doubleClick(grip)
    expect(done).toHaveBeenLastCalledWith(WIDE.sidebar)
  })
})
