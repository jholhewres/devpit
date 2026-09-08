import { describe, expect, it } from 'vitest'
import type { LayoutNode } from '../gen/bindings'
import { leafIds } from './tree'

const tree: LayoutNode = {
  type: 'split',
  direction: 'horizontal',
  ratio: 0.5,
  first: {
    type: 'leaf',
    id: 'leaf_a',
    tmuxTarget: 'session_a:main',
    kind: 'terminal',
    agent: 'none'
  },
  second: {
    type: 'split',
    direction: 'vertical',
    ratio: 0.5,
    first: {
      type: 'leaf',
      id: 'leaf_b',
      tmuxTarget: 'session_b:main',
      kind: 'terminal',
      agent: 'none'
    },
    second: {
      type: 'leaf',
      id: 'leaf_c',
      tmuxTarget: 'session_c:main',
      kind: 'terminal',
      agent: 'none'
    }
  }
}

describe('session tree', () => {
  it('lists nested leaves in visual order', () => {
    expect(leafIds(tree)).toEqual(['leaf_a', 'leaf_b', 'leaf_c'])
  })
})
