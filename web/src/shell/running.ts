import type { PaneRunning } from '../gen/bindings'
import type { Tab } from './strip'
import type { AgentState, Doing } from './useAgents'

/*
 * What a tab is running, out of what the whole project is running.
 *
 * `session.running` answers per pane, and the strip and the sidebar both ask
 * per tab. Between them sits a tree: a tab has one pane until it is split and
 * two afterwards, and the question "is an agent open in this tab" has to keep
 * meaning the same thing across that.
 *
 * The rule when a split disagrees with itself is that the **agent wins**. A
 * tab with Claude in one half and a build in the other is a tab you would call
 * "Claude", and stopping the agent is the costlier surprise of the two — so it
 * is also the one the close prompt has to be about.
 */

/** Everything this tab has in front of it, in the order the panes are drawn. */
export function inTab(
  running: readonly PaneRunning[],
  tab: Tab | null | undefined,
): readonly PaneRunning[] {
  if (!tab?.panes?.length) return []
  const byPane = new Map(running.map((one) => [one.paneId, one]))
  return tab.panes.flatMap((paneId) => {
    const one = byPane.get(paneId)
    return one ? [one] : []
  })
}

/** The agent open in this tab, if one is. */
export function agentIn(
  running: readonly PaneRunning[],
  tab: Tab | null | undefined,
): PaneRunning | null {
  return inTab(running, tab).find((one) => one.agent !== null) ?? null
}

/**
 * What this tab has in front of it worth naming: the agent, or whatever else
 * is running when nothing is an agent.
 *
 * A shell at its prompt is not it. That is the pane doing nothing, and a row
 * that says `zsh` is a row saying "idle" in a way nobody reads as idle.
 */
export function busyIn(
  running: readonly PaneRunning[],
  tab: Tab | null | undefined,
): PaneRunning | null {
  const here = inTab(running, tab)
  return here.find((one) => one.agent !== null) ?? here.find((one) => one.busy) ?? null
}

/**
 * What a close has to warn about, or nothing when the tab is idle.
 *
 * `agent`, `command` and `unsaved` rather than a boolean, because the three
 * closes are different sentences: stopping an agent mid-task, stopping a
 * build, and throwing away what somebody typed are not the same loss, and a
 * prompt that words them alike teaches people to click through it.
 */
export type Stops = {
  readonly kind: 'agent' | 'command' | 'unsaved'
  readonly label: string
}

export function stopsOnClose(
  running: readonly PaneRunning[],
  tab: Tab | null | undefined,
): Stops | null {
  const here = busyIn(running, tab)
  if (!here) return null
  return { kind: here.agent ? 'agent' : 'command', label: here.label }
}

/**
 * What this tab's agent last said it was doing, if it said anything.
 *
 * Absent for an agent the app did not start: what it is doing is something
 * only it can report, and it has only been asked to report it when it was
 * launched from here. A missing answer is missing, not idle.
 */
export function doingIn(
  running: readonly PaneRunning[],
  doing: Doing,
  tab: Tab | null | undefined,
): AgentState | null {
  const here = agentIn(running, tab)
  return here ? (doing[here.paneId] ?? null) : null
}

/** Whether two answers from the process table say the same thing. */
export function same(was: readonly PaneRunning[], now: readonly PaneRunning[]): boolean {
  return (
    was.length === now.length &&
    was.every((one, at) => {
      const other = now[at]
      return (
        other !== undefined &&
        one.paneId === other.paneId &&
        one.command === other.command &&
        one.busy === other.busy &&
        one.agent === other.agent &&
        one.label === other.label
      )
    })
  )
}
