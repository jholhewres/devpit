import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it, vi } from 'vitest'

import { openCardTerminal } from './useCardActs'

const launched = vi.fn()

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    sessionLaunchAgent: (...args: unknown[]) => launched(...args),
    cardTerminal: () => ({
      layout: { projectId: 'p1', focusedId: 'leaf_1', tree: { type: 'leaf', id: 'leaf_1' } },
      tabId: 'tab_named_by_the_backend',
      cardId: 'card_1',
    }),
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ show: vi.fn() }) }))

describe("a card's terminal tab", () => {
  it('is filed under the tab and card the backend answers with', async () => {
    const show = vi.fn()
    expect(await openCardTerminal('p1', 'card_1', 'Wire the board', show, 'claude')).toBeNull()
    expect(show).toHaveBeenCalledWith('term', {
      id: 'tab_named_by_the_backend',
      title: 'Wire the board',
      cardId: 'card_1',
      launch: 'claude',
    })
  })

  it('starts the agent in the pane it has when the tab is already open', async () => {
    const show = vi.fn()
    const open = [{ id: 'tab_named_by_the_backend', kind: 'term' as const }]
    expect(await openCardTerminal('p1', 'card_1', 'Wire the board', show, 'claude', open)).toBeNull()
    expect(launched).toHaveBeenCalledWith('p1', 'leaf_1', 'claude', null)
    expect(show).toHaveBeenCalledWith('term', { id: 'tab_named_by_the_backend', title: 'Wire the board', cardId: 'card_1' })
  })

  it('never has its id spelled by the window', () => {
    const src = resolve(process.cwd(), 'src')
    const spelled = (readdirSync(src, { recursive: true }) as string[])
      .filter((path) => /\.tsx?$/.test(path) && !/\.test\.tsx?$/.test(path))
      .filter((path) => readFileSync(resolve(src, path), 'utf8').includes('tab_card_'))
    expect(spelled).toEqual([])
  })
})
