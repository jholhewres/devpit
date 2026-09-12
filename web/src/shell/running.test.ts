import { describe, expect, it } from 'vitest'

import type { PaneRunning } from '../gen/bindings'
import { agentIn, busyIn, inTab, stopsOnClose } from './running'
import type { Tab } from './strip'

const pane = (paneId: string, over: Partial<PaneRunning> = {}): PaneRunning => ({
  paneId,
  command: 'zsh',
  busy: false,
  agent: null,
  label: 'zsh',
  ...over,
})

const claude = (paneId: string): PaneRunning =>
  pane(paneId, { command: 'claude', busy: true, agent: 'claude', label: 'Claude Code' })

const build = (paneId: string): PaneRunning =>
  pane(paneId, { command: 'cargo', busy: true, label: 'cargo' })

const tab = (panes?: readonly string[]): Tab => ({ id: 't1', kind: 'term', ...(panes && { panes }) })

describe('what a tab is running', () => {
  it('reads the panes the tab owns, in drawing order', () => {
    const running = [build('b'), claude('a')]
    expect(inTab(running, tab(['a', 'b'])).map((one) => one.paneId)).toEqual(['a', 'b'])
  })

  it('is nothing for a tab that has not opened its panes yet', () => {
    expect(inTab([claude('a')], tab())).toEqual([])
    expect(agentIn([claude('a')], null)).toBeNull()
  })

  /* The bug this replaces: the sidebar matched one recorded pane id against
     the leaves, the tab never recorded one, and a terminal with Claude open
     showed nothing at all. */
  it('finds the agent through the tab, not through a single remembered pane', () => {
    expect(agentIn([claude('a')], tab(['a']))?.label).toBe('Claude Code')
  })

  it('ignores panes belonging to another tab', () => {
    expect(agentIn([claude('other')], tab(['a']))).toBeNull()
  })

  it('lets the agent win a split that disagrees with itself', () => {
    const running = [build('a'), claude('b')]
    expect(busyIn(running, tab(['a', 'b']))?.agent).toBe('claude')
  })

  it('names whatever is running when none of it is an agent', () => {
    expect(busyIn([build('a')], tab(['a']))?.label).toBe('cargo')
  })

  it('says nothing is running when every pane is at its prompt', () => {
    expect(busyIn([pane('a'), pane('b')], tab(['a', 'b']))).toBeNull()
  })
})

describe('what closing a tab would stop', () => {
  it('is nothing for an idle tab, so the close stays a single click', () => {
    expect(stopsOnClose([pane('a')], tab(['a']))).toBeNull()
  })

  it('words an agent and a command differently', () => {
    expect(stopsOnClose([claude('a')], tab(['a']))).toEqual({
      kind: 'agent',
      label: 'Claude Code',
    })
    expect(stopsOnClose([build('a')], tab(['a']))).toEqual({ kind: 'command', label: 'cargo' })
  })

  /* Stopping an agent mid-task is the costlier surprise, so a mixed split has
     to raise the agent's wording rather than the build's. */
  it('warns about the agent when a split is running both', () => {
    expect(stopsOnClose([build('a'), claude('b')], tab(['a', 'b']))?.kind).toBe('agent')
  })
})
