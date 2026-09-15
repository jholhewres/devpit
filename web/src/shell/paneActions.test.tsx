import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { LayoutNode, PaneRunning } from '../gen/bindings'
import { usePaneActions } from './paneActions'
import type { Tab } from './strip'

const calls: string[] = []

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    sessionCloseLeaf: (_project: string, _tab: string, leaf: string) => {
      calls.push(`closeLeaf ${leaf}`)
      return { projectId: 'prj', focusedId: 'leaf_2', tree: { type: 'leaf', id: 'leaf_2' } }
    },
    cliInstallations: () => [{ directory: '/home/me/.claude-glm', profiles: ['glm'], default: false }],
    agentProfiles: () => [{ id: 'prof_glm', label: 'glm', reach: 'runnable', driver: 'claude', mine: true }],
    chatAdopt: (_project: string, session: string, profile: string, _title: string | null, card: string | null) => {
      calls.push(`adopt ${session} as ${profile}${card ? ` for ${card}` : ''}`)
      return 'conv_9'
    },
  },
}))

const shell = {
  project: { id: 'prj' },
  running: [] as PaneRunning[],
  close: vi.fn((id: string) => calls.push(`close ${id}`)),
  closeNow: vi.fn((id: string) => calls.push(`closeNow ${id}`)),
  show: vi.fn((kind: string, tab?: { id: string }) => calls.push(`show ${kind} ${tab?.id}`)),
  agentSessions: {} as Record<string, { sessionId: string; installation: string }>,
}
vi.mock('./useShell', () => ({ useShell: () => shell }))

afterEach(() => {
  calls.length = 0
  shell.running = []
  shell.agentSessions = {}
})

const tab: Tab = { id: 'term_1', kind: 'term', panes: ['leaf_1', 'leaf_2'] }
const one = { type: 'leaf', id: 'leaf_1' } as unknown as LayoutNode
const split = {
  type: 'split',
  id: 'split_1',
  direction: 'horizontal',
  first: one,
  second: { type: 'leaf', id: 'leaf_2' },
} as unknown as LayoutNode

const actions = (tree: LayoutNode, onLayout = vi.fn(), onNotice = vi.fn()) =>
  renderHook(() => usePaneActions({ tab, tree, focused: 'leaf_1', onLayout, onNotice }))

describe('closing a terminal pane', () => {
  it('closes only the pane in front when the tab is split', async () => {
    const onLayout = vi.fn()
    const { result } = actions(split, onLayout)
    expect(result.current.closeLabel).toBe('Close pane')
    act(() => result.current.closePane())
    await waitFor(() => expect(onLayout).toHaveBeenCalled())
    expect(calls).toEqual(['closeLeaf leaf_1'])
  })

  it('closes the tab, through its own prompt, when it is the last pane', () => {
    const { result } = actions(one)
    act(() => result.current.closePane())
    expect(calls).toEqual(['close term_1'])
  })

  /* A pane with an agent in it is not stopped by one stray click. */
  it('asks for a second press before stopping what runs in a split pane', async () => {
    shell.running = [{ paneId: 'leaf_1', command: 'claude', busy: true, agent: 'claude', label: 'Claude Code' }]
    const { result } = actions(split)
    act(() => result.current.closePane())
    expect(calls).toEqual([])
    expect(result.current.armed).toBe(true)
    expect(result.current.closeLabel).toBe('Press again to stop Claude Code')
    act(() => result.current.closePane())
    await waitFor(() => expect(calls).toEqual(['closeLeaf leaf_1']))
  })
})

describe('continuing a terminal conversation in a chat', () => {
  it('is only offered when the pane’s session is known', () => {
    expect(actions(one).result.current.toChat).toBeNull()
  })

  it('adopts the session under the profile of its installation, stops the pane, then opens the chat', async () => {
    shell.agentSessions = { leaf_1: { sessionId: 'abc', installation: '/home/me/.claude-glm' } }
    const { result } = actions(split)
    act(() => result.current.toChat?.())
    await waitFor(() => expect(calls).toEqual(['adopt abc as prof_glm', 'closeLeaf leaf_1', 'show chat conv_9']))
  })

  it('files the conversation under the card when the pane is a card’s', async () => {
    shell.agentSessions = { leaf_1: { sessionId: 'abc', installation: '/home/me/.claude-glm' } }
    const { result } = renderHook(() =>
      usePaneActions({ tab: { ...tab, cardId: 'card_1' }, tree: split, focused: 'leaf_1', onLayout: vi.fn(), onNotice: vi.fn() }),
    )
    act(() => result.current.toChat?.())
    await waitFor(() => expect(calls[0]).toBe('adopt abc as prof_glm for card_1'))
  })

  it('closes a single-pane tab without asking, since moving the agent was the ask', async () => {
    shell.agentSessions = { leaf_1: { sessionId: 'abc', installation: '/home/me/.claude-glm' } }
    const { result } = actions(one)
    act(() => result.current.toChat?.())
    await waitFor(() => expect(calls).toEqual(['adopt abc as prof_glm', 'closeNow term_1', 'show chat conv_9']))
  })

  it('says so and touches nothing when no profile runs that installation', async () => {
    shell.agentSessions = { leaf_1: { sessionId: 'abc', installation: '/somewhere/else' } }
    const onNotice = vi.fn()
    const { result } = actions(split, vi.fn(), onNotice)
    act(() => result.current.toChat?.())
    await waitFor(() => expect(onNotice).toHaveBeenCalled())
    expect(calls).toEqual([])
  })
})
