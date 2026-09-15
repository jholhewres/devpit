import { describe, expect, it, vi } from 'vitest'

import type { CardSession } from '../gen/bindings'
import { anyLive, liveBody, liveWork, stopLiveWork } from './liveWork'

const called = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    sessionCloseTab: (...args: unknown[]) => called('sessionCloseTab', ...args),
    runCancel: (...args: unknown[]) => called('runCancel', ...args),
  },
}))

const session = (over: Partial<CardSession>): CardSession => ({
  kind: 'pane',
  ref: 'leaf_1',
  state: null,
  tabId: null,
  leafId: null,
  runId: null,
  ...over,
})

describe('the work still going on a card', () => {
  it('is its terminals with an agent in front and its runs in flight', () => {
    const live = liveWork([
      session({ ref: 'leaf_1', state: 'waiting', tabId: 'tab_1', leafId: 'leaf_1' }),
      session({ ref: 'leaf_2', state: 'open', tabId: 'tab_1', leafId: 'leaf_2' }),
      session({ ref: 'leaf_3', state: null, tabId: 'tab_1', leafId: 'leaf_3' }),
      session({ kind: 'run', ref: 's-run', state: 'working', runId: 'run_1' }),
      session({ kind: 'run', ref: 's-old', state: 'done', runId: 'run_0' }),
      session({ kind: 'background', ref: 's-bg', state: 'working' }),
    ])
    expect(live).toEqual({ tabs: ['tab_1'], runs: ['run_1'] })
    expect(anyLive(live)).toBe(true)
    expect(liveBody(live)).toBe(
      'Work is still going on this card: an agent is in its terminal and 1 run is still going. Close its terminal and stop its runs first?',
    )
  })

  it('is nothing for a terminal with only a shell and finished runs', () => {
    const live = liveWork([session({ state: null, tabId: 'tab_1', leafId: 'leaf_1' }), session({ kind: 'run', state: 'failed', runId: 'run_1' })])
    expect(anyLive(live)).toBe(false)
  })

  it('is stopped by closing the terminal and cancelling the runs', async () => {
    const closeNow = vi.fn()
    expect(await stopLiveWork('p1', 'card_1', { tabs: ['tab_1'], runs: ['run_1'] }, closeNow)).toBeNull()
    expect(called.mock.calls).toEqual([
      ['sessionCloseTab', 'p1', 'tab_1'],
      ['runCancel', 'card_1', 'run_1'],
    ])
    expect(closeNow).toHaveBeenCalledWith('tab_1')
  })
})
