import { describe, expect, it } from 'vitest'

import type { LayoutNode } from '../gen/bindings'
import { boundedRatio, draggedTo, leaves, nodeAt, paneCount, sides, LEAST } from './splits'

const leaf = (id: string): LayoutNode =>
  ({ type: 'leaf', id, tmuxTarget: `s:${id}`, kind: 'terminal', agent: 'none', title: '' })

const split = (id: string, first: LayoutNode, second: LayoutNode, ratio = 0.5): LayoutNode =>
  ({ type: 'split', id, direction: 'horizontal', ratio, first, second })

const tree = split('sp1', leaf('a'), split('sp2', leaf('b'), leaf('c')))

describe('reading a layout tree', () => {
  it('lists the leaves in the order they are drawn', () => {
    expect(leaves(tree)).toEqual(['a', 'b', 'c'])
    expect(paneCount(tree)).toBe(3)
  })

  it('finds a node by id, split or leaf', () => {
    expect(nodeAt(tree, 'sp2')?.type).toBe('split')
    expect(nodeAt(tree, 'b')?.type).toBe('leaf')
    expect(nodeAt(tree, 'nope')).toBeNull()
  })

  it('counts a lone pane as one', () => {
    expect(paneCount(leaf('a'))).toBe(1)
  })
})

describe('where a boundary lands', () => {
  it('follows the pointer down the box', () => {
    expect(draggedTo(300, 1000, 0.5)).toBeCloseTo(0.3)
  })

  it('never leaves a side too small to use', () => {
    /* A pane at two per cent has a terminal one column wide, and the shell
       inside it wraps every character. */
    expect(draggedTo(1, 1000, 0.5)).toBe(LEAST)
    expect(draggedTo(999, 1000, 0.5)).toBe(1 - LEAST)
  })

  it('keeps what it had when the box has no size yet', () => {
    /* The frame after a split, before layout. Dividing by zero would persist
       a NaN into the tree. */
    expect(draggedTo(10, 0, 0.42)).toBe(0.42)
  })

  it('bounds a ratio that arrives out of range', () => {
    expect(boundedRatio(-1)).toBe(LEAST)
    expect(boundedRatio(2)).toBe(1 - LEAST)
  })
})

describe('the two sides as CSS reads them', () => {
  it('splits a half down the middle', () => {
    expect(sides(0.5)).toEqual({ first: '50.0000%', second: '50.0000%' })
  })

  it('always adds to a whole', () => {
    for (const ratio of [0.1, 0.25, 0.8, 0.92]) {
      const { first, second } = sides(ratio)
      expect(parseFloat(first) + parseFloat(second)).toBeCloseTo(100)
    }
  })
})
