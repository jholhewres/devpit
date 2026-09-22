import type { Tab } from './strip'

/*
 * The terminals of projects you have just left, kept mounted and hidden.
 *
 * Switching project used to unmount every terminal and build them again on
 * the way back: a new xterm, a new WebGL context, the scrollback replayed and
 * the stream attached again — a dark pane for as long as that took, every
 * time. A kept terminal is still there, hidden the way a tab behind another
 * tab already is, and coming back to it costs nothing.
 *
 * Bounded by projects rather than by tabs: each one holds a WebGL context,
 * and a webview gives out only so many before it starts taking them back.
 */

export const KEPT_PROJECTS = 3

export interface KeptTab {
  readonly projectId: string
  readonly tab: Tab
}

/** What stays mounted once `projectId` shows `open`: its own kept tabs first,
 *  then those of the projects visited just before it, up to `most` projects. */
export function keptAfter(
  was: readonly KeptTab[],
  projectId: string | null,
  open: readonly Tab[],
  keeps: (tab: Tab) => boolean,
  most = KEPT_PROJECTS,
): readonly KeptTab[] {
  const mine = projectId ? open.filter(keeps).map((tab) => ({ projectId, tab })) : []
  const all = [...mine, ...was.filter((one) => one.projectId !== projectId)]
  const recent = [...new Set(all.map((one) => one.projectId))].slice(0, most)
  const next = all.filter((one) => recent.includes(one.projectId))
  const same =
    next.length === was.length &&
    next.every((one, at) => one.projectId === was[at]?.projectId && one.tab === was[at]?.tab)
  return same ? was : next
}
