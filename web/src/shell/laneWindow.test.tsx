import { act, cleanup, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Card, Column } from '../gen/bindings'
import { LaneCards } from './LaneCards'
import type { Drag } from './useDrag'
import { LANE_PAGE } from './useShown'

/*
 * A lane of hundreds of cards drew every one of them at once. It draws a
 * page, and the next when its end comes near the view.
 */

afterEach(cleanup)

let near: ((seen: { isIntersecting: boolean }[]) => void) | null = null
beforeEach(() => {
  near = null
  vi.stubGlobal(
    'IntersectionObserver',
    class {
      constructor(callback: typeof near) {
        near = callback
      }
      observe(): void {}
      disconnect(): void {}
    },
  )
})

const column: Column = { id: 'col_1', name: 'Ship', position: 0, step: null, onPass: null, autonomy: 'manual', role: null, roleChosen: false }

const card = (at: number): Card => ({
  id: `card_${at}`,
  columnId: 'col_1',
  title: `Card ${at}`,
  body: '',
  position: at,
  worktreePath: null,
  dueAt: null,
  dueTime: false,
  costUsd: null,
  comments: 0,
  pinned: 0,
  runs: [],
  activity: null,
})

const idle: Drag = { held: null, landing: null, landed: null, down: () => undefined, move: () => undefined, up: () => ({ what: 'nothing' }) }

function lane(count: number, drag: Drag = idle, dropping = false) {
  return render(
    <LaneCards
      lane={{ column, cards: Array.from({ length: count }, (_, at) => card(at)) }}
      others={[]}
      onMove={() => undefined}
      drag={drag}
      dropping={dropping}
      progress={{}}
      onOpen={() => undefined}
      onPlay={async () => null}
      onRename={() => undefined}
      acts={() => ({}) as never}
    />,
  )
}

const tiles = (container: HTMLElement) => container.querySelectorAll('.blane__list > .tile').length

describe('a long lane', () => {
  it('draws a page of 200 cards first, and says how long that took', () => {
    const started = performance.now()
    const { container } = lane(200)
    const took = performance.now() - started
    console.info(`a lane of 200 cards drew ${tiles(container)} tiles in ${took.toFixed(1)} ms`)
    expect(tiles(container)).toBe(LANE_PAGE)
    expect(container.querySelector('.blane__more')).toBeTruthy()
  })

  it('draws the next page when its end comes near, until every card is drawn', () => {
    const { container } = lane(70)
    act(() => near!([{ isIntersecting: true }]))
    expect(tiles(container)).toBe(2 * LANE_PAGE)
    act(() => near!([{ isIntersecting: true }]))
    expect(tiles(container)).toBe(70)
    expect(container.querySelector('.blane__more')).toBeNull()
  })

  it('a short lane draws every card and no mark', () => {
    const { container } = lane(5)
    expect(tiles(container)).toBe(5)
    expect(container.querySelector('.blane__more')).toBeNull()
  })

  it('takes a card dropped past what it drew, right after the last card drawn', () => {
    const held = { card: card(999), height: 40, moved: true } as unknown as NonNullable<Drag['held']>
    const dragging: Drag = { ...idle, held, landing: { lane: 'col_1', index: LANE_PAGE } }
    const { container } = lane(200, dragging, true)
    const list = container.querySelector('.blane__list')!
    const slot = list.querySelector('.slot')
    expect(slot).toBeTruthy()
    expect(slot!.previousElementSibling?.getAttribute('data-card')).toBe(`card_${LANE_PAGE - 1}`)
  })
})
