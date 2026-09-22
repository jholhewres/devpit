import { describe, expect, it } from 'vitest'

import { keptAfter, type KeptTab } from './kept'
import type { Tab } from './strip'

const term = (id: string): Tab => ({ id, kind: 'term' })
const chat = (id: string): Tab => ({ id, kind: 'chat' })
const isTerm = (tab: Tab) => tab.kind === 'term'
const ids = (kept: readonly KeptTab[]) => kept.map((one) => `${one.projectId}/${one.tab.id}`)

describe('the terminals kept across a project switch', () => {
  it('keeps the project you left, and only its terminals', () => {
    const inA = keptAfter([], 'a', [term('t1'), chat('c1')], isTerm)
    const inB = keptAfter(inA, 'b', [term('t2')], isTerm)
    expect(ids(inB)).toEqual(['b/t2', 'a/t1'])
  })

  it('forgets a terminal its own project closed', () => {
    const inA = keptAfter([], 'a', [term('t1'), term('t2')], isTerm)
    expect(ids(keptAfter(inA, 'a', [term('t2')], isTerm))).toEqual(['a/t2'])
  })

  it('drops the project visited longest ago', () => {
    let kept: readonly KeptTab[] = []
    for (const project of ['a', 'b', 'c', 'd']) {
      kept = keptAfter(kept, project, [term(`t${project}`)], isTerm, 3)
    }
    expect(ids(kept)).toEqual(['d/td', 'c/tc', 'b/tb'])
  })

  it('answers with the same list when nothing changed', () => {
    const open = [term('t1')]
    const kept = keptAfter([], 'a', open, isTerm)
    expect(keptAfter(kept, 'a', open, isTerm)).toBe(kept)
  })
})
