import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { LayoutNode } from '../gen/bindings'
import { Split } from './Split'

afterEach(cleanup)

const leaf = (id: string): LayoutNode =>
  ({ type: 'leaf', id, tmuxTarget: `s:${id}`, kind: 'terminal', agent: 'none', title: '' })

const split = (id: string, first: LayoutNode, second: LayoutNode, over = {}): LayoutNode =>
  ({ type: 'split', id, direction: 'horizontal', ratio: 0.5, first, second, ...over })

const draw = (node: LayoutNode, over: Partial<Parameters<typeof Split>[0]> = {}) =>
  render(
    <Split
      node={node}
      focused="a"
      onFocus={() => {}}
      onRatio={() => {}}
      leaf={(id) => <span data-testid={`leaf-${id}`}>{id}</span>}
      {...over}
    />,
  )

describe('drawing a tab as its tree says', () => {
  it('draws every leaf, not just the focused one', () => {
    /* A terminal unmounted to draw its sibling loses its attachment and its
       scrollback, so a split you can only half see is not a split. */
    draw(split('sp1', leaf('a'), split('sp2', leaf('b'), leaf('c'))))
    expect(screen.getByTestId('leaf-a')).toBeTruthy()
    expect(screen.getByTestId('leaf-b')).toBeTruthy()
    expect(screen.getByTestId('leaf-c')).toBeTruthy()
  })

  it('names the focused leaf so the edge can say so', () => {
    const { container } = draw(split('sp1', leaf('a'), leaf('b')), { focused: 'b' })
    const focused = container.querySelectorAll('.tleaf[data-focused="true"]')
    expect(focused).toHaveLength(1)
    expect(focused[0]!.textContent).toBe('b')
  })

  it('gives a boundary per split, oriented as the split says', () => {
    const { container } = draw(
      split('sp1', leaf('a'), leaf('b'), { direction: 'vertical' }),
    )
    const bar = container.querySelector('[role="separator"]')
    expect(bar?.getAttribute('aria-orientation')).toBe('horizontal')
  })

  it('splits the box by the ratio it was given', () => {
    const { container } = draw(split('sp1', leaf('a'), leaf('b'), { ratio: 0.25 }))
    /* Compared as numbers: the DOM normalises `25.0000%` to `25%`, and the
       trailing zeros are only there to keep a third of a box off the end of a
       float. */
    const sides = container.querySelectorAll('.split__side')
    expect(parseFloat((sides[0] as HTMLElement).style.flexBasis)).toBeCloseTo(25)
    expect(parseFloat((sides[1] as HTMLElement).style.flexBasis)).toBeCloseTo(75)
  })

  it('falls back to half for a tree that predates ratios', () => {
    const { container } = draw(split('sp1', leaf('a'), leaf('b'), { ratio: null }))
    expect(
      parseFloat((container.querySelector('.split__side') as HTMLElement).style.flexBasis),
    ).toBeCloseTo(50)
  })

  it('tells the owner which leaf was pressed', () => {
    const onFocus = vi.fn()
    draw(split('sp1', leaf('a'), leaf('b')), { onFocus })
    fireEvent.mouseDown(screen.getByTestId('leaf-b'))
    expect(onFocus).toHaveBeenCalledWith('b')
  })

  it('reports a boundary only once it settles', () => {
    /* Persisting on every pointer move would be a write per frame. */
    const onRatio = vi.fn()
    const { container } = draw(split('sp1', leaf('a'), leaf('b')), { onRatio })
    const bar = container.querySelector('[role="separator"]')!
    fireEvent.pointerDown(bar)
    fireEvent.pointerMove(window, { clientX: 100, clientY: 0 })
    expect(onRatio).not.toHaveBeenCalled()
    fireEvent.pointerUp(window)
    expect(onRatio).toHaveBeenCalledTimes(1)
    expect(onRatio.mock.calls[0]![0]).toBe('sp1')
  })

  it('forgets a drag on a boundary with no id rather than addressing nothing', () => {
    const onRatio = vi.fn()
    const { container } = draw(split('', leaf('a'), leaf('b')), { onRatio })
    fireEvent.pointerDown(container.querySelector('[role="separator"]')!)
    fireEvent.pointerMove(window, { clientX: 100, clientY: 0 })
    fireEvent.pointerUp(window)
    expect(onRatio).not.toHaveBeenCalled()
  })

  it('draws a lone leaf with no boundary at all', () => {
    const { container } = draw(leaf('a'))
    expect(container.querySelector('[role="separator"]')).toBeNull()
  })
})
