import { act, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { NEAR, ownScroll, useFollow, type Follow } from './useFollow'

/* jsdom lays nothing out, so the scroller's size is set by hand and a resize
   is announced the way the browser would, through the observer. */
let resized: () => void = () => {}
beforeEach(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      constructor(callback: () => void) {
        resized = callback
      }
      observe(): void {}
      disconnect(): void {}
    },
  )
})
afterEach(() => vi.unstubAllGlobals())

function scroller(): { follow: () => Follow<HTMLDivElement>; box: HTMLDivElement; grow: (to: number) => void; userTo: (top: number) => void } {
  let follow: Follow<HTMLDivElement> | null = null
  function Pane(): React.JSX.Element {
    follow = useFollow<HTMLDivElement>(0)
    return (
      <div ref={follow.box}>
        <div />
      </div>
    )
  }
  const { container } = render(<Pane />)
  const box = container.firstElementChild as HTMLDivElement
  let height = 1000
  Object.defineProperty(box, 'clientHeight', { value: 200 })
  Object.defineProperty(box, 'scrollHeight', { get: () => height })
  // The app's own scroll arrives as an event, as it would in a browser.
  let top = 0
  Object.defineProperty(box, 'scrollTop', {
    get: () => top,
    set: (to: number) => {
      top = to
      box.dispatchEvent(new Event('scroll'))
    },
  })
  return {
    follow: () => follow!,
    box,
    grow: (to) => {
      height = to
      act(() => resized())
    },
    userTo: (to) => {
      top = to
      act(() => void box.dispatchEvent(new Event('scroll')))
    },
  }
}

describe('ownScroll', () => {
  it('knows the app’s own scroll and forgets it once seen', () => {
    const marks = [300, 800]
    expect(ownScroll(marks, 801)).toBe(true)
    expect(marks).toEqual([])
    expect(ownScroll(marks, 800)).toBe(false)
  })
})

describe('useFollow', () => {
  it('keeps up with growth that is not a new message', () => {
    const pane = scroller()
    pane.grow(1500)
    expect(pane.box.scrollTop).toBe(1300)
    pane.grow(2000)
    expect(pane.box.scrollTop).toBe(1800)
  })

  it('stays where the person scrolled, and offers the way back', () => {
    const pane = scroller()
    pane.grow(1500)
    pane.userTo(600)
    pane.grow(2000)
    expect(pane.box.scrollTop).toBe(600)
    expect(pane.follow().away).toBe(true)
  })

  it('follows again once the person is back at the end', () => {
    const pane = scroller()
    pane.grow(1500)
    pane.userTo(600)
    pane.userTo(1300)
    pane.grow(2000)
    expect(pane.box.scrollTop).toBe(1800)
    expect(pane.follow().away).toBe(false)
  })

  it('does not follow just short of the end', () => {
    const pane = scroller()
    pane.grow(1500)
    pane.userTo(1300 - NEAR / 2)
    pane.grow(2000)
    expect(pane.box.scrollTop).toBe(1300 - NEAR / 2)
  })

  it('jumps to the end and follows from there', () => {
    const pane = scroller()
    pane.grow(1500)
    pane.userTo(100)
    act(() => pane.follow().toEnd())
    expect(pane.box.scrollTop).toBe(1300)
    pane.grow(1700)
    expect(pane.box.scrollTop).toBe(1500)
  })
})
